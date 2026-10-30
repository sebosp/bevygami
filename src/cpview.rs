//! The crease pattern of the sheet, unfolded, in a corner of the window: the creases made so far
//! in the colors of the CP editors, the fold line of the current step in the line of the
//! diagrams, and the part of the sheet that moves, lightened like the flap in 3D.
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

use crate::fold::{Face, FoldKind};
use crate::session::Session;
use crate::settings::Settings;
use crate::theme::{crease_on_paper, hex};

pub const SIZE: u32 = 220;
pub const SWATCH_W: u32 = 44;
pub const SWATCH_H: u32 = 12;

#[derive(Resource)]
pub struct CpImages {
    pub pattern: Handle<Image>,
    /// The legend: a valley and a mountain line.
    pub valley: Handle<Image>,
    pub mountain: Handle<Image>,
}

pub struct CpViewPlugin;

impl Plugin for CpViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup)
            .add_systems(Update, draw.after(crate::session::SessionSystems));
    }
}

fn blank(w: u32, h: u32) -> Image {
    Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut swatch = |kind: FoldKind| {
        let mut pixmap = Pixmap::new(SWATCH_W, SWATCH_H).expect("swatch size");
        let y = SWATCH_H as f32 / 2.0;
        line(
            &mut pixmap,
            (2.0, y),
            (SWATCH_W as f32 - 2.0, y),
            crease_on_paper(kind),
            2.5,
            Some(kind),
            Transform::identity(),
        );
        let mut image = blank(SWATCH_W, SWATCH_H);
        image.data = Some(pixmap.take());
        images.add(image)
    };
    let valley = swatch(FoldKind::Valley);
    let mountain = swatch(FoldKind::Mountain);
    commands.insert_resource(CpImages {
        pattern: images.add(blank(SIZE, SIZE)),
        valley,
        mountain,
    });
}

fn paint(color: Color) -> Paint<'static> {
    let c = color.to_srgba();
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8,
        (c.alpha * 255.0) as u8,
    );
    paint.anti_alias = true;
    paint
}

/// A line, dashed like the diagrams for a fold to come.
fn line(
    pixmap: &mut Pixmap,
    a: (f32, f32),
    b: (f32, f32),
    color: Color,
    width: f32,
    dashed: Option<FoldKind>,
    transform: Transform,
) {
    let mut pb = PathBuilder::new();
    pb.move_to(a.0, a.1);
    pb.line_to(b.0, b.1);
    let Some(path) = pb.finish() else {
        return;
    };
    let dash = match dashed {
        Some(FoldKind::Valley) => StrokeDash::new(vec![7.0, 4.5], 0.0),
        Some(FoldKind::Mountain) => StrokeDash::new(vec![7.0, 3.0, 1.2, 3.0, 1.2, 3.0], 0.0),
        None => None,
    };
    let stroke = Stroke {
        width,
        line_cap: LineCap::Butt,
        dash,
        ..default()
    };
    pixmap.stroke_path(&path, &paint(color), &stroke, transform, None);
}

fn polygon(pixmap: &mut Pixmap, face: &Face, color: Color, transform: Transform) {
    let mut pb = PathBuilder::new();
    for (i, p) in face.poly.iter().enumerate() {
        if i == 0 {
            pb.move_to(p.x as f32, p.y as f32);
        } else {
            pb.line_to(p.x as f32, p.y as f32);
        }
    }
    pb.close();
    if let Some(path) = pb.finish() {
        pixmap.fill_path(&path, &paint(color), FillRule::Winding, transform, None);
    }
}

fn draw(
    session: Res<Session>,
    settings: Res<Settings>,
    cp: Res<CpImages>,
    mut images: ResMut<Assets<Image>>,
    mut drawn: Local<Option<(u32, usize, usize)>>,
) {
    let key = (session.generation, settings.paper as usize, settings.theme as usize);
    if *drawn == Some(key) {
        return;
    }
    *drawn = Some(key);
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("pattern size");
    // Paper coordinates to pixels, the sheet turned like the lesson starts.
    let half = SIZE as f32 / 2.0;
    let diamond = session.current().diamond;
    let scale = (half - 8.0) / if diamond { std::f32::consts::SQRT_2 } else { 1.0 };
    let turn = if diamond { 45.0 } else { 0.0 };
    // y up on the paper, down in the image.
    let transform = Transform::from_scale(scale, -scale)
        .post_rotate_at(-turn, 0.0, 0.0)
        .post_translate(half, half);
    let sheet = Face {
        poly: crate::fold::Paper::square(crate::fold::Pose::IDENTITY).faces[0]
            .poly
            .clone(),
        pose: crate::fold::Pose::IDENTITY,
        layer: 0,
    };
    polygon(&mut pixmap, &sheet, hex(settings.paper.hex()), transform);
    let paper = session.shown();
    if let Some(plan) = session.plan() {
        for (face, moving) in plan.before.faces.iter().zip(&plan.moving) {
            if *moving {
                polygon(&mut pixmap, face, Color::WHITE.with_alpha(0.45), transform);
            }
        }
    }
    // The widths are in pixels: the transform scales the points, not the strokes.
    let to_px = |p: bevy::math::DVec2| {
        let mut point = tiny_skia::Point::from_xy(p.x as f32, p.y as f32);
        transform.map_point(&mut point);
        (point.x, point.y)
    };
    let edge = hex("#6b6f80");
    let corners = &sheet.poly;
    for i in 0..4 {
        line(
            &mut pixmap,
            to_px(corners[i]),
            to_px(corners[(i + 1) % 4]),
            edge,
            1.5,
            None,
            Transform::identity(),
        );
    }
    for crease in &paper.creases {
        let age = session.step.saturating_sub(crease.step + 1) as f32;
        let alpha = (1.0 - 0.15 * age).max(0.45);
        line(
            &mut pixmap,
            to_px(crease.a),
            to_px(crease.b),
            crease_on_paper(crease.kind).with_alpha(alpha),
            2.0,
            None,
            Transform::identity(),
        );
    }
    if let Some(plan) = session.plan() {
        for chord in &plan.chords {
            line(
                &mut pixmap,
                to_px(chord.a),
                to_px(chord.b),
                crease_on_paper(chord.kind),
                3.0,
                Some(chord.kind),
                Transform::identity(),
            );
        }
    }
    if let Some(mut image) = images.get_mut(&cp.pattern) {
        image.data = Some(pixmap.take());
    }
}
