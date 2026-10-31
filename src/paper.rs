//! The paper in 3D: a mesh per face and side, turned around the fold line while it folds, and the
//! lines of the diagrams drawn on it with gizmos.
//!
//! - the creases of the steps done, thin and solid, fainter the older they are;
//! - the fold line of the current step, dashed (valley) or dash-dot-dot (mountain), breathing;
//! - the arrow of the fold, full head for a valley, half head for a mountain;
//! - the dotted outline where the flap lands, the X-ray line of the diagrams.
use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::color::Mix;
use bevy::math::{Affine3A, DMat2, DVec2, DVec3};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::sync::Arc;

use crate::bend::{Bent, Samples, Sheet};
use crate::fold::{Face, FoldKind, Paper, Planned, Style, shared_edges};
use crate::motion::Reverse;
use crate::session::{Session, SessionSystems};
use crate::settings::Settings;
use crate::theme::{self, ThemeMode, crease_on_paper, hex};

/// The height between two layers of paper.
pub const LAYER_GAP: f32 = 0.006;
/// The lines float this far over the paper, under the next layer.
const LINE_LIFT: f32 = 0.0018;

#[derive(Component)]
pub struct FaceMesh {
    pub face: usize,
    /// The meshes of the two sides and the points of the mesh, for the faces that may bend.
    sides: Option<(Handle<Mesh>, Handle<Mesh>)>,
    bent: bool,
}

/// The lines of the creases done.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct CreaseGizmos;

/// The fold line of the current step.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FoldLineGizmos;

/// The arrows and the landing outline, seen through the paper.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct GuideGizmos;

#[derive(Resource)]
struct PaperMaterials {
    front: Handle<StandardMaterial>,
    back: Handle<StandardMaterial>,
    front_moving: Handle<StandardMaterial>,
    back_moving: Handle<StandardMaterial>,
}

/// Where each face of [`Session::shown`] is now.
#[derive(Resource, Default)]
pub struct Placement {
    pub faces: Vec<Affine3A>,
    generation: u32,
    /// The faces meshed for bending, when the step bends the paper, see [`crate::bend`].
    sheet: Option<Arc<Sheet>>,
    /// Where the paper bends now.
    bent: Option<Bent>,
}

impl Placement {
    /// The point `p` of the face (paper coordinates) in the world.
    pub fn point(&self, face: usize, p: DVec2) -> Option<Vec3> {
        let affine = self.faces.get(face)?;
        let mut at = to_world(affine, p);
        if let (Some(sheet), Some(bent)) = (&self.sheet, &self.bent) {
            at += bent.at(sheet, face, p).as_vec3();
        }
        Some(at)
    }

    fn bends(&self, face: usize) -> bool {
        self.bent.as_ref().is_some_and(|b| !b.flat(face))
    }
}

/// The creases on the edges of the shown faces: face, segment, kind on the front, step.
#[derive(Resource, Default)]
struct CreaseMarks(Vec<(usize, DVec2, DVec2, FoldKind, usize)>);

pub struct PaperPlugin;

impl Plugin for PaperPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<CreaseGizmos>()
            .init_gizmo_group::<FoldLineGizmos>()
            .init_gizmo_group::<GuideGizmos>()
            .init_resource::<Placement>()
            .init_resource::<CreaseMarks>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    respawn,
                    place,
                    breathe,
                    paint_paper,
                    line_widths,
                    draw_creases,
                    draw_fold_line,
                    draw_guides,
                )
                    .chain()
                    .after(SessionSystems),
            );
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut configs: ResMut<GizmoConfigStore>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let paper = |color: Color| StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.95,
        reflectance: 0.15,
        ..default()
    };
    commands.insert_resource(PaperMaterials {
        front: materials.add(paper(hex("#f2c14e"))),
        back: materials.add(paper(hex("#f6f3ec"))),
        front_moving: materials.add(paper(hex("#f2c14e"))),
        back_moving: materials.add(paper(hex("#f6f3ec"))),
    });
    // A light from the front and a weaker one from behind: both sides of the paper read.
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            ..default()
        },
        Transform::from_xyz(1.5, 2.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3500.0,
            ..default()
        },
        Transform::from_xyz(-1.0, -2.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    ambient.brightness = 900.0;

    let (config, _) = configs.config_mut::<CreaseGizmos>();
    config.line.width = 2.0;
    let (config, _) = configs.config_mut::<FoldLineGizmos>();
    config.line.width = 4.0;
    let (config, _) = configs.config_mut::<GuideGizmos>();
    config.line.width = 2.5;
    config.depth_bias = -1.0;
}

/// The front and the back of a face, in paper coordinates.
fn face_meshes(face: &Face) -> (Mesh, Mesh) {
    let positions: Vec<[f32; 3]> = face.poly.iter().map(|p| [p.x as f32, p.y as f32, 0.0]).collect();
    // The paper coordinates as UVs, for a texture later.
    let uvs: Vec<[f32; 2]> = face
        .poly
        .iter()
        .map(|p| [(p.x as f32 + 1.0) / 2.0, (1.0 - p.y as f32) / 2.0])
        .collect();
    let n = positions.len() as u32;
    let fan: Vec<u32> = (1..n - 1).flat_map(|i| [0, i, i + 1]).collect();
    let back_fan: Vec<u32> = (1..n - 1).flat_map(|i| [0, i + 1, i]).collect();
    let mesh = |normal: [f32; 3], indices: Vec<u32>| {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal; n as usize])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone())
            .with_inserted_indices(Indices::U32(indices))
    };
    (mesh([0.0, 0.0, 1.0], fan), mesh([0.0, 0.0, -1.0], back_fan))
}

/// A face meshed finely, the meshes it bends with.
fn sample_meshes(samples: &Samples) -> (Mesh, Mesh) {
    let positions: Vec<[f32; 3]> = samples.points.iter().map(|p| [p.x as f32, p.y as f32, 0.0]).collect();
    let uvs: Vec<[f32; 2]> = samples
        .points
        .iter()
        .map(|p| [(p.x as f32 + 1.0) / 2.0, (1.0 - p.y as f32) / 2.0])
        .collect();
    let front: Vec<u32> = samples.triangles.iter().flat_map(|[a, b, c]| [*a, *b, *c]).collect();
    let back: Vec<u32> = samples.triangles.iter().flat_map(|[a, b, c]| [*a, *c, *b]).collect();
    let n = positions.len();
    let mesh = |normal: [f32; 3], indices: Vec<u32>| {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal; n])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone())
            .with_inserted_indices(Indices::U32(indices))
    };
    (mesh([0.0, 0.0, 1.0], front), mesh([0.0, 0.0, -1.0], back))
}

/// The points of the mesh of a face moved by the bend (local coordinates), and their normals.
fn bend_mesh(samples: &Samples, offset: impl Fn(DVec2) -> Vec3) -> (Vec<[f32; 3]>, Vec<Vec3>) {
    let positions: Vec<Vec3> = samples
        .points
        .iter()
        .map(|p| Vec3::new(p.x as f32, p.y as f32, 0.0) + offset(*p))
        .collect();
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for [a, b, c] in &samples.triangles {
        let (a, b, c) = (*a as usize, *b as usize, *c as usize);
        let n = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
        for v in [a, b, c] {
            normals[v] += n;
        }
    }
    let normals = normals.into_iter().map(|n| n.normalize_or(Vec3::Z)).collect();
    (positions.into_iter().map(|p| p.to_array()).collect(), normals)
}

/// The faces of the shown paper spawned again when the step changes.
fn respawn(
    session: Res<Session>,
    mut placement: ResMut<Placement>,
    mut marks: ResMut<CreaseMarks>,
    faces: Query<Entity, With<FaceMesh>>,
    materials: Res<PaperMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    if placement.generation == session.generation && !placement.faces.is_empty() {
        return;
    }
    placement.generation = session.generation;
    for entity in &faces {
        commands.entity(entity).despawn();
    }
    let paper = session.shown();
    let moving = session.plan().map(|p| p.moving.clone()).unwrap_or_default();
    let bends = session
        .plan()
        .is_some_and(|p| p.reverse.as_ref().is_some_and(|r| r.bending));
    placement.sheet = bends.then(|| Arc::new(Sheet::new(paper)));
    placement.bent = None;
    for (i, face) in paper.faces.iter().enumerate() {
        let (front, back) = match &placement.sheet {
            Some(sheet) => sample_meshes(&sheet.faces[i]),
            None => face_meshes(face),
        };
        let (front, back) = (meshes.add(front), meshes.add(back));
        let m = moving.get(i).copied().unwrap_or(false);
        let (front_material, back_material) = if m {
            (materials.front_moving.clone(), materials.back_moving.clone())
        } else {
            (materials.front.clone(), materials.back.clone())
        };
        commands.spawn((
            FaceMesh {
                face: i,
                sides: bends.then(|| (front.clone(), back.clone())),
                bent: false,
            },
            Transform::default(),
            Visibility::default(),
            children![
                (Mesh3d(front), MeshMaterial3d(front_material)),
                (Mesh3d(back), MeshMaterial3d(back_material)),
            ],
        ));
    }
    placement.faces = vec![Affine3A::IDENTITY; paper.faces.len()];
    marks.0 = crease_marks(paper);
}

fn crease_marks(paper: &Paper) -> Vec<(usize, DVec2, DVec2, FoldKind, usize)> {
    let mut marks = Vec::new();
    for (i, face) in paper.faces.iter().enumerate() {
        for crease in &paper.creases {
            let line = Face {
                poly: vec![crease.a, crease.b, crease.a],
                pose: face.pose,
                layer: 0,
            };
            for (a, b) in shared_edges(face, &line) {
                if !marks
                    .iter()
                    .any(|(f, p, q, _, _): &(usize, DVec2, DVec2, FoldKind, usize)| {
                        *f == i && p.distance(a) < 1e-6 && q.distance(b) < 1e-6
                    })
                {
                    marks.push((i, a, b, crease.kind, crease.step));
                }
            }
        }
    }
    marks
}

/// The isometry of a face at rest, its layer as height.
fn rest(face: &Face) -> Affine3A {
    let pose = face.pose;
    let flip = if pose.flipped() {
        Quat::from_rotation_x(PI)
    } else {
        Quat::IDENTITY
    };
    let rotation = Quat::from_rotation_z(pose.angle() as f32) * flip;
    let translation = Vec3::new(pose.t.x as f32, pose.t.y as f32, face.layer as f32 * LAYER_GAP);
    Affine3A::from_rotation_translation(rotation, translation)
}

/// Turns the moving faces around the fold line and lifts them to their layer at the end.
fn place(
    session: Res<Session>,
    mut placement: ResMut<Placement>,
    mut faces: Query<(&mut FaceMesh, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let paper = session.shown();
    let angle = session.angle();
    for (i, face) in paper.faces.iter().enumerate() {
        let mut affine = rest(face);
        if let Some(plan) = session.plan()
            && let Some(reverse) = &plan.reverse
            && session.t != 0.0
        {
            affine = reverse_motion(plan, reverse, i, session.t);
        } else if let Some(plan) = session.plan()
            && plan.moving[i]
            && plan.fold.style == Style::Collapse
            && session.t != 0.0
        {
            affine = collapse_motion(face, &plan.landed.faces[i], plan.turns[i], session.t);
        } else if let Some(plan) = session.plan()
            && plan.moving[i]
            && angle != 0.0
        {
            let (a, _) = plan.axis;
            let a = Vec3::new(a.x as f32, a.y as f32, 0.0);
            let side = Vec3::new(plan.side.x as f32, plan.side.y as f32, 0.0);
            // The moving side turns up (valley) or down (mountain).
            let axis = side.cross(Vec3::Z).normalize();
            let signed = if plan.turns[i] == FoldKind::Valley {
                angle
            } else {
                -angle
            };
            let turn = Affine3A::from_translation(a)
                * Affine3A::from_axis_angle(axis, signed)
                * Affine3A::from_translation(-a);
            // Half a turn mirrors the height of the layer, it ends at its new layer.
            let z_before = face.layer as f32 * LAYER_GAP;
            let z_after = plan.landed.faces[i].layer as f32 * LAYER_GAP;
            let lift = Affine3A::from_translation(Vec3::Z * (angle / PI) * (z_after + z_before));
            affine = lift * turn * affine;
        }
        if let Some(slot) = placement.faces.get_mut(i) {
            *slot = affine;
        }
    }
    // Where stiff paper would tear, it bends.
    placement.bent = match (session.plan(), &placement.sheet) {
        (Some(plan), Some(sheet)) if session.t != 0.0 => plan.reverse.as_ref().map(|reverse| {
            let pose = reverse.at(session.t as f64);
            let faces = &plan.before.faces;
            sheet.bend(&|i, u| {
                let q = faces[i].pose.apply(u) - reverse.vertex;
                pose.of(reverse.parts[i]) * DVec3::new(q.x, q.y, 0.0)
            })
        }),
        _ => None,
    };
    for (mut face, mut transform) in &mut faces {
        let Some(affine) = placement.faces.get(face.face) else {
            continue;
        };
        *transform = Transform::from_matrix(Mat4::from(*affine));
        let Some((front, back)) = face.sides.clone() else {
            continue;
        };
        let bends = placement.bends(face.face);
        if !bends && !face.bent {
            continue;
        }
        let (Some(sheet), i) = (&placement.sheet, face.face) else {
            continue;
        };
        // The bend is in the world, the mesh in the face's own coordinates.
        let to_local = affine.matrix3.inverse();
        let (positions, normals) = bend_mesh(&sheet.faces[i], |p| match &placement.bent {
            Some(bent) if bends => to_local * bent.at(sheet, i, p).as_vec3(),
            _ => Vec3::ZERO,
        });
        for (handle, side) in [(front, 1.0), (back, -1.0)] {
            if let Some(mut mesh) = meshes.get_mut(&handle) {
                let normals: Vec<[f32; 3]> = normals.iter().map(|n| (*n * side).to_array()).collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            }
        }
        face.bent = bends;
    }
}

/// A face of a reverse fold on its way, see [`Reverse`]: turned around the vertex, in the middle
/// plane between the halves of the flap, and lifted to its new layer at the end.
fn reverse_motion(plan: &Planned, reverse: &Reverse, i: usize, t: f32) -> Affine3A {
    let face = &plan.before.faces[i];
    let layers = || {
        plan.before
            .faces
            .iter()
            .zip(&plan.moving)
            .filter(|(_, m)| **m)
            .map(|(f, _)| f.layer)
    };
    let (lo, hi) = (layers().min().unwrap_or(0), layers().max().unwrap_or(0));
    let middle = (lo + hi) as f32 / 2.0 * LAYER_GAP;
    let pivot = Vec3::new(reverse.vertex.x as f32, reverse.vertex.y as f32, middle);
    let turn = reverse.at(t as f64).of(reverse.parts[i]).as_mat3();
    let z_before = face.layer as f32 * LAYER_GAP;
    let z_after = plan.landed.faces[i].layer as f32 * LAYER_GAP;
    // The flap ends turned over around the middle plane, the layers that stay where they were.
    let z_end = if plan.moving[i] {
        2.0 * middle - z_before
    } else {
        z_before
    };
    Affine3A::from_translation(Vec3::Z * t * (z_after - z_end))
        * Affine3A::from_translation(pivot)
        * Affine3A::from_mat3(turn)
        * Affine3A::from_translation(-pivot)
        * rest(face)
}

/// A face of a collapse on its way, `t` from 0 to 1: the isometry from where it was to where it
/// lands, as a screw in 3D. A reflection is a half turn around its axis (a glide slides along it),
/// a rotation turns in the plane, lifted over the paper on the way.
fn collapse_motion(from: &Face, to: &Face, toward: FoldKind, t: f32) -> Affine3A {
    let delta = from.pose.inverse().then(&to.pose);
    let (z0, z1) = (from.layer as f32 * LAYER_GAP, to.layer as f32 * LAYER_GAP);
    let side = if toward == FoldKind::Valley { 1.0 } else { -1.0 };
    let screw = if delta.m.determinant() < 0.0 {
        // The axis: the direction the reflection keeps, through half the offset across it.
        let a = delta.m.x_axis.y.atan2(delta.m.x_axis.x) / 2.0;
        let u = DVec2::new(a.cos(), a.sin());
        let n = DVec2::new(-u.y, u.x);
        let along = delta.t.dot(u) as f32;
        let point = n * (delta.t.dot(n) / 2.0);
        let point = Vec3::new(point.x as f32, point.y as f32, 0.0);
        let u3 = Vec3::new(u.x as f32, u.y as f32, 0.0);
        // The half turn goes over the paper toward the viewer, or under it.
        let turn_axis = if u3.cross(Vec3::Z).dot(Vec3::new(n.x as f32, n.y as f32, 0.0)) > 0.0 {
            u3
        } else {
            -u3
        };
        let z_fix = t * (z1 + z0);
        Affine3A::from_translation(u3 * along * t + Vec3::Z * z_fix)
            * Affine3A::from_translation(point)
            * Affine3A::from_axis_angle(turn_axis, side * PI * t)
            * Affine3A::from_translation(-point)
    } else {
        let angle = delta.m.x_axis.y.atan2(delta.m.x_axis.x);
        let lift = Vec3::Z * ((z1 - z0) * t + side * 0.25 * (PI * t).sin());
        if angle.abs() < 1e-9 {
            Affine3A::from_translation(Vec3::new(delta.t.x as f32, delta.t.y as f32, 0.0) * t + lift)
        } else {
            // The center of the rotation: (I - M) c = t.
            let c = (DMat2::IDENTITY - delta.m).inverse() * delta.t;
            let c = Vec3::new(c.x as f32, c.y as f32, 0.0);
            Affine3A::from_translation(lift + c)
                * Affine3A::from_rotation_z(angle as f32 * t)
                * Affine3A::from_translation(-c)
        }
    };
    screw * rest(from)
}

/// From 0 to 1 and back, every 2.4 seconds; 1 when the motion is reduced.
pub fn breath(time: &Time, settings: &Settings) -> f32 {
    if settings.reduce_motion {
        1.0
    } else {
        0.5 + 0.5 * (time.elapsed_secs() * TAU / 2.4).sin()
    }
}

/// The flap that will move glows a little, in time with the fold line.
fn breathe(
    time: Res<Time>,
    settings: Res<Settings>,
    session: Res<Session>,
    paper_materials: Res<PaperMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let front = hex(settings.paper.hex());
    let back = hex("#f6f3ec");
    let glow = if session.at_rest() && !session.finished() {
        0.10 * breath(&time, &settings)
    } else {
        0.0
    };
    let accent = theme::scene(ThemeMode::Light).accent;
    if let Some(mut m) = materials.get_mut(&paper_materials.front_moving) {
        m.base_color = front.mix(&Color::WHITE, glow);
        m.emissive = LinearRgba::from(accent) * glow * 0.6;
    }
    if let Some(mut m) = materials.get_mut(&paper_materials.back_moving) {
        m.base_color = back.mix(&accent, glow * 0.4);
    }
}

/// The widths of the lines follow the settings, in pixels.
fn line_widths(settings: Res<Settings>, mut configs: ResMut<GizmoConfigStore>) {
    if !settings.is_changed() {
        return;
    }
    let scale = settings.lines.scale();
    configs.config_mut::<CreaseGizmos>().0.line.width = 3.0 * scale;
    configs.config_mut::<FoldLineGizmos>().0.line.width = 6.0 * scale;
    configs.config_mut::<GuideGizmos>().0.line.width = 3.0 * scale;
}

/// The front color follows the settings.
fn paint_paper(
    settings: Res<Settings>,
    paper_materials: Res<PaperMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !settings.is_changed() {
        return;
    }
    if let Some(mut m) = materials.get_mut(&paper_materials.front) {
        m.base_color = hex(settings.paper.hex());
    }
}

fn to_world(affine: &Affine3A, p: DVec2) -> Vec3 {
    affine.transform_point3(Vec3::new(p.x as f32, p.y as f32, 0.0))
}

/// A crease on both sides of its face: on the back, a mountain is a valley. On a face that bends,
/// in pieces that follow it.
fn both_sides(
    placement: &Placement,
    face: usize,
    a: DVec2,
    b: DVec2,
    kind: FoldKind,
    mut draw: impl FnMut(Vec3, Vec3, FoldKind),
) {
    let Some(affine) = placement.faces.get(face) else {
        return;
    };
    let normal = affine.transform_vector3(Vec3::Z) * LINE_LIFT;
    let pieces = if placement.bends(face) { 12 } else { 1 };
    let at = |k: usize| {
        let p = a.lerp(b, k as f64 / pieces as f64);
        placement.point(face, p).unwrap_or_else(|| to_world(affine, p))
    };
    for k in 0..pieces {
        let (pa, pb) = (at(k), at(k + 1));
        draw(pa + normal, pb + normal, kind);
        draw(pa - normal, pb - normal, kind.opposite());
    }
}

fn draw_creases(
    session: Res<Session>,
    placement: Res<Placement>,
    marks: Res<CreaseMarks>,
    mut gizmos: Gizmos<CreaseGizmos>,
) {
    let step = session.step;
    for (face, a, b, kind, made) in &marks.0 {
        // The last fold done reads best, the older ones fade.
        let age = step.saturating_sub(*made + 1) as f32;
        let alpha = (0.8 - 0.1 * age).max(0.4);
        both_sides(&placement, *face, *a, *b, *kind, |p, q, kind| {
            gizmos.line(p, q, crease_on_paper(kind).with_alpha(alpha));
        });
    }
}

/// Dashes along a segment: the pattern of lengths, on and off in turns.
fn dashes(p: Vec3, q: Vec3, pattern: &[f32], mut draw: impl FnMut(Vec3, Vec3)) {
    let length = p.distance(q);
    if length < 1e-6 {
        return;
    }
    let dir = (q - p) / length;
    // Centered: the pattern is cut evenly at both ends.
    let period: f32 = pattern.iter().sum();
    let mut s = -((length % period) / 2.0);
    let mut i = 0;
    while s < length {
        let len = pattern[i % pattern.len()];
        if i % 2 == 0 {
            let (from, to) = (s.max(0.0), (s + len).min(length));
            if to > from {
                draw(p + dir * from, p + dir * to);
            }
        }
        s += len;
        i += 1;
    }
}

const VALLEY_DASH: [f32; 2] = [0.07, 0.045];
const MOUNTAIN_DASH: [f32; 6] = [0.07, 0.03, 0.012, 0.03, 0.012, 0.03];

fn draw_fold_line(
    time: Res<Time>,
    settings: Res<Settings>,
    session: Res<Session>,
    placement: Res<Placement>,
    mut gizmos: Gizmos<FoldLineGizmos>,
) {
    let Some(plan) = session.plan() else {
        return;
    };
    let alpha = 0.7 + 0.3 * breath(&time, &settings);
    for chord in &plan.chords {
        both_sides(&placement, chord.face, chord.a, chord.b, chord.kind, |p, q, kind| {
            let pattern: &[f32] = if kind == FoldKind::Valley {
                &VALLEY_DASH
            } else {
                &MOUNTAIN_DASH
            };
            let color = crease_on_paper(kind).with_alpha(alpha);
            dashes(p, q, pattern, |a, b| gizmos.line(a, b, color));
        });
    }
}

/// The arrow from the moving point to where it lands, and the outline of the landed flap.
fn draw_guides(time: Res<Time>, settings: Res<Settings>, session: Res<Session>, mut gizmos: Gizmos<GuideGizmos>) {
    let Some(plan) = session.plan() else {
        return;
    };
    if !session.at_rest() {
        return;
    }
    let scene = theme::scene(settings.theme);
    let alpha = 0.6 + 0.4 * breath(&time, &settings);
    let top = (session.shown().max_layer() + 1) as f32 * LAYER_GAP;
    if settings.landing {
        // The edges of the landed flap that no other moving face shares: its outline.
        let moving: Vec<&Face> = plan
            .landed
            .faces
            .iter()
            .zip(&plan.moving)
            .filter(|(_, m)| **m)
            .map(|(f, _)| f)
            .collect();
        for (k, face) in moving.iter().enumerate() {
            let n = face.poly.len();
            for e in 0..n {
                let (a, b) = (face.poly[e], face.poly[(e + 1) % n]);
                let edge = Face {
                    poly: vec![a, b, a],
                    pose: face.pose,
                    layer: 0,
                };
                let shared: f64 = moving
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != k)
                    .flat_map(|(_, g)| shared_edges(&edge, g))
                    .map(|(p, q)| p.distance(q))
                    .sum();
                if shared > a.distance(b) - 1e-6 {
                    continue;
                }
                let (pa, pb) = (face.pose.apply(a), face.pose.apply(b));
                let z = (face.layer + 1) as f32 * LAYER_GAP;
                let p = Vec3::new(pa.x as f32, pa.y as f32, z);
                let q = Vec3::new(pb.x as f32, pb.y as f32, z);
                dashes(p, q, &[0.012, 0.03], |a, b| {
                    gizmos.line(a, b, scene.dim.with_alpha(0.9))
                });
            }
        }
    }
    // How the gripped flap turns: the front half of a reverse fold leads.
    let grip_turn = plan
        .before
        .faces
        .iter()
        .enumerate()
        .filter(|(i, f)| plan.moving[*i] && f.contains(plan.grip))
        .max_by_key(|(_, f)| f.layer)
        .map(|(i, _)| plan.turns[i])
        .unwrap_or(plan.fold.kind);
    if settings.arrows
        && let (Some(from), Some(to)) = (plan.before.image_of(plan.grip), plan.landed.image_of(plan.grip))
    {
        let (from, to) = (
            Vec3::new(from.x as f32, from.y as f32, top),
            Vec3::new(to.x as f32, to.y as f32, top),
        );
        let span = from.distance(to);
        // An arc over the paper for a valley, under it for a mountain; the ends stay off the
        // points.
        let lift = if grip_turn == FoldKind::Valley { 1.0 } else { -1.0 } * 0.3 * span;
        // Bent sideways too, along the fold line: a curve even seen from the front.
        let (_, d) = plan.axis;
        let along = Vec3::new(d.x as f32, d.y as f32, 0.0) * 0.18 * span;
        let point = |s: f32| from.lerp(to, s) + (Vec3::Z * lift + along) * (PI * s).sin();
        let (s0, s1) = (0.06, 0.94);
        let segments = 32;
        let color = scene.guide.with_alpha(alpha);
        let points: Vec<Vec3> = (0..=segments)
            .map(|i| point(s0 + (s1 - s0) * i as f32 / segments as f32))
            .collect();
        gizmos.linestrip(points.iter().copied(), color);
        let head = |tip: Vec3, before: Vec3, full: bool, gizmos: &mut Gizmos<GuideGizmos>| {
            let back = (before - tip).normalize_or_zero();
            let across = back.cross(Vec3::Z).normalize_or_zero();
            let size = 0.09;
            gizmos.line(tip, tip + (back * 0.9 + across * 0.45) * size, color);
            if full {
                gizmos.line(tip, tip + (back * 0.9 - across * 0.45) * size, color);
            }
        };
        let valley = grip_turn == FoldKind::Valley;
        head(points[segments], points[segments - 3], valley, &mut gizmos);
        if plan.fold.unfold {
            // Fold and unfold: the arrow goes and comes back, a head at both ends.
            head(points[0], points[3], valley, &mut gizmos);
        }
    }
}
