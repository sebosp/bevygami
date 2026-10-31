//! The camera around the paper and the hands on it: a drag on the flap that moves folds it, a
//! drag anywhere else (or with the right button) turns the paper in 3D, the wheel or a pinch
//! zooms. The keys: → Space Enter fold, ← Backspace unfold, R resets the view.
use std::f32::consts::PI;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::math::DVec2;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::cursor::Hand;
use crate::fold::{Pose, cross};
use crate::session::{Animation, Command, Session, SessionSystems};
use crate::ui::Panel;

#[derive(Component)]
pub struct MainCamera;

/// The camera looks at the center of the paper from `distance`, turned by `rotation`.
#[derive(Resource)]
pub struct Orbit {
    pub rotation: Quat,
    pub distance: f32,
}

impl Orbit {
    /// Facing the paper, a little from below, like a sheet on a table.
    pub fn home() -> Quat {
        Quat::from_rotation_x(0.3)
    }
}

impl Default for Orbit {
    fn default() -> Self {
        Orbit {
            rotation: Orbit::home(),
            distance: 6.2,
        }
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct ResetView;

/// The pointer read, before the cursor is chosen.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewSystems;

/// How far around a flap a press still takes it, in pixels.
const REACH: f32 = 14.0;

/// What the pointer holds.
#[derive(Default)]
enum Hold {
    #[default]
    Nothing,
    /// Over a panel or a button.
    Ui,
    Orbit,
    /// The point taken on the flap and where it lands, on the folded plane; the press position.
    Flap {
        grab: DVec2,
        land: DVec2,
        pressed: Vec2,
        moved: bool,
    },
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Orbit>()
            .add_message::<ResetView>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    keys,
                    pointer.before(SessionSystems).in_set(ViewSystems),
                    zoom,
                    reset_view,
                    move_camera,
                )
                    .chain(),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    // Smooth edges for the paper and the lines.
    commands.spawn((Camera3d::default(), Msaa::Sample4, MainCamera, Transform::default()));
}

fn keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    panel: Res<Panel>,
    mut commands: MessageWriter<Command>,
    mut reset: MessageWriter<ResetView>,
) {
    if *panel != Panel::None {
        return;
    }
    if keyboard.any_just_pressed([KeyCode::ArrowRight, KeyCode::Space, KeyCode::Enter]) {
        commands.write(Command::Fold);
    }
    if keyboard.any_just_pressed([KeyCode::ArrowLeft, KeyCode::Backspace]) {
        commands.write(Command::Back);
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        reset.write(ResetView);
    }
}

/// Where the ray under the pointer meets the plane of the paper.
fn on_paper(camera: &Camera, camera_transform: &GlobalTransform, position: Vec2) -> Option<DVec2> {
    let ray = camera.viewport_to_world(camera_transform, position).ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Z))?;
    let p = ray.get_point(distance);
    Some(DVec2::new(p.x as f64, p.y as f64))
}

/// The pointer: the mouse, or the first finger on a touch screen.
struct Pointer {
    id: PointerId,
    position: Vec2,
    pressed: bool,
    released: bool,
    delta: Vec2,
}

fn current_pointer(
    window: &Window,
    buttons: &ButtonInput<MouseButton>,
    motion: &AccumulatedMouseMotion,
    touches: &Touches,
) -> Option<Pointer> {
    if let Some(touch) = touches.iter().next() {
        if touches.iter().count() > 1 {
            return None;
        }
        return Some(Pointer {
            id: PointerId::Touch(touch.id()),
            position: touch.position(),
            pressed: touches.just_pressed(touch.id()),
            released: false,
            delta: touch.delta(),
        });
    }
    if let Some(touch) = touches.iter_just_released().next() {
        return Some(Pointer {
            id: PointerId::Touch(touch.id()),
            position: touch.position(),
            pressed: false,
            released: true,
            delta: Vec2::ZERO,
        });
    }
    let held = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
    Some(Pointer {
        id: PointerId::Mouse,
        position: window.cursor_position()?,
        pressed: buttons.any_just_pressed(held),
        released: buttons.any_just_released(held) && !buttons.any_pressed(held),
        delta: motion.delta,
    })
}

#[allow(clippy::too_many_arguments)]
fn pointer(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    touches: Res<Touches>,
    hover: Res<HoverMap>,
    nodes: Query<(), With<Node>>,
    panel: Res<Panel>,
    mut orbit: ResMut<Orbit>,
    mut session: ResMut<Session>,
    mut hand: ResMut<Hand>,
    mut hold: Local<Hold>,
) {
    let Some(pointer) = current_pointer(&window, &buttons, &motion, &touches) else {
        // Two fingers: the pinch zooms, see `zoom`.
        *hold = Hold::Nothing;
        hand.set_if_neq(Hand::Free);
        return;
    };
    let (camera, camera_transform) = *camera;
    // A press this far from a flap, on the paper, still takes it.
    let reach = reach(camera, camera_transform, pointer.position);
    let over_ui = hover
        .get(&pointer.id)
        .is_some_and(|hits| hits.keys().any(|e| nodes.contains(*e)));
    if pointer.pressed {
        *hold = if over_ui || *panel != Panel::None {
            Hold::Ui
        } else if buttons.just_pressed(MouseButton::Left) || matches!(pointer.id, PointerId::Touch(_)) {
            grab_flap(
                &session,
                on_paper(camera, camera_transform, pointer.position),
                pointer.position,
                reach,
            )
            .unwrap_or(Hold::Orbit)
        } else {
            Hold::Orbit
        };
        if matches!(*hold, Hold::Flap { .. }) {
            session.dragging = true;
        }
    }
    match &mut *hold {
        Hold::Orbit if pointer.delta != Vec2::ZERO => {
            // Turns around the camera's own axes, the paper follows the hand.
            let speed = 0.008;
            let yaw = Quat::from_axis_angle(orbit.rotation * Vec3::Y, -pointer.delta.x * speed);
            let pitch = Quat::from_axis_angle(orbit.rotation * Vec3::X, -pointer.delta.y * speed);
            orbit.rotation = (yaw * pitch * orbit.rotation).normalize();
        }
        Hold::Flap {
            grab,
            land,
            pressed,
            moved,
        } => {
            if pointer.position.distance(*pressed) > 6.0 {
                *moved = true;
            }
            if *moved && let Some(at) = on_paper(camera, camera_transform, pointer.position) {
                let span = *land - *grab;
                let p = ((at - *grab).dot(span) / span.length_squared()).clamp(0.0, 1.0);
                // The point goes over a half circle: its shadow on the paper moves like a cosine.
                let turn = (1.0 - 2.0 * p).acos() as f32 / PI;
                let unfold = session.plan().is_some_and(|p| p.fold.unfold);
                session.t = if unfold { turn / 2.0 } else { turn };
            }
        }
        _ => {}
    }
    if pointer.released {
        if let Hold::Flap { moved, .. } = *hold {
            session.dragging = false;
            let unfold = session.plan().is_some_and(|p| p.fold.unfold);
            let half = if unfold { 0.25 } else { 0.5 };
            // A tap folds, a drag past half way too.
            let target = if !moved || session.t > half { 1.0 } else { 0.0 };
            session.animation = Some(Animation { target });
        }
        *hold = Hold::Nothing;
    }
    let now = match *hold {
        Hold::Flap { .. } => Hand::Folding,
        Hold::Orbit => Hand::Turning,
        Hold::Ui => Hand::Free,
        Hold::Nothing if over_ui || *panel != Panel::None || matches!(pointer.id, PointerId::Touch(_)) => Hand::Free,
        Hold::Nothing => {
            let at = on_paper(camera, camera_transform, pointer.position);
            if grab_flap(&session, at, pointer.position, reach).is_some() {
                Hand::OverFlap
            } else {
                Hand::OverTurn
            }
        }
    };
    hand.set_if_neq(now);
}

/// [`REACH`] pixels on the paper, around the pointer.
fn reach(camera: &Camera, camera_transform: &GlobalTransform, position: Vec2) -> f64 {
    let here = on_paper(camera, camera_transform, position);
    let there = on_paper(camera, camera_transform, position + Vec2::new(REACH, 0.0));
    here.zip(there).map_or(0.0, |(a, b)| a.distance(b))
}

/// The closest point of the polygon's boundary to `p`.
fn closest_on(poly: &[DVec2], p: DVec2) -> DVec2 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let ab = b - a;
            a + ab * ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
        })
        .min_by(|x, y| x.distance(p).total_cmp(&y.distance(p)))
        .unwrap_or(p)
}

/// The flap of the current fold under the pointer, at rest, or within `reach` of it.
fn grab_flap(session: &Session, at: Option<DVec2>, pressed: Vec2, reach: f64) -> Option<Hold> {
    let mut at = at?;
    if !session.at_rest() {
        return None;
    }
    let plan = session.plan()?;
    let inside = |face: &crate::fold::Face, p: DVec2| {
        let image = face.image();
        let n = image.len();
        (0..n).all(|i| cross(image[(i + 1) % n] - image[i], p - image[i]) * orientation(face.pose) >= -1e-9)
    };
    let flaps = || {
        plan.before
            .faces
            .iter()
            .zip(&plan.moving)
            .filter(|(_, m)| **m)
            .map(|(f, _)| f)
    };
    if !flaps().any(|face| inside(face, at)) {
        // Next to the flap: the point of its edge, a hair inside.
        let (face, near) = flaps()
            .map(|face| (face, closest_on(&face.image(), at)))
            .min_by(|x, y| x.1.distance(at).total_cmp(&y.1.distance(at)))?;
        if near.distance(at) > reach {
            return None;
        }
        at = near + (face.centroid_image() - near).normalize_or_zero() * 1e-4;
    }
    // Where the point taken lands: the face under it, from where it is to where it goes.
    let landing = |p: DVec2| {
        let i = plan
            .before
            .faces
            .iter()
            .enumerate()
            .filter(|(i, f)| plan.moving[*i] && f.contains(f.pose.inverse().apply(p)))
            .max_by_key(|(_, f)| f.layer)
            .map(|(i, _)| i)?;
        let paper = plan.before.faces[i].pose.inverse().apply(p);
        Some(plan.landed.faces[i].pose.apply(paper))
    };
    let mut grab = at;
    let mut land = landing(at)?;
    if (land - grab).length() < 0.3 {
        // Taken next to the fold line, the moving point steers better.
        grab = plan.before.image_of(plan.grip)?;
        land = plan.landed.image_of(plan.grip)?;
    }
    Some(Hold::Flap {
        grab,
        land,
        pressed,
        moved: false,
    })
}

/// The image of a counterclockwise polygon turns clockwise when the face is flipped.
fn orientation(pose: Pose) -> f64 {
    if pose.flipped() { -1.0 } else { 1.0 }
}

fn zoom(
    scroll: Res<AccumulatedMouseScroll>,
    touches: Res<Touches>,
    hover: Res<HoverMap>,
    nodes: Query<(), With<Node>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut orbit: ResMut<Orbit>,
) {
    // Ctrl and the wheel zoom the panels, see `crate::zoom`.
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let over_ui = ctrl
        || hover
            .get(&PointerId::Mouse)
            .is_some_and(|hits| hits.keys().any(|e| nodes.contains(*e)));
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    let mut factor = if over_ui { 1.0 } else { 0.9f32.powf(lines) };
    let fingers: Vec<_> = touches.iter().collect();
    if let [a, b] = fingers[..] {
        let now = a.position().distance(b.position());
        let before = a.previous_position().distance(b.previous_position());
        if now > 1.0 && before > 1.0 {
            factor *= before / now;
        }
    }
    orbit.distance = (orbit.distance * factor).clamp(2.0, 12.0);
}

/// R and the button reset the view, the first frame too: a narrow window (a phone held upright)
/// needs the camera further away.
fn reset_view(
    mut resets: MessageReader<ResetView>,
    mut orbit: ResMut<Orbit>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut started: Local<bool>,
) {
    if resets.read().count() == 0 && *started {
        return;
    }
    *started = true;
    let aspect = window.width() / window.height().max(1.0);
    *orbit = Orbit {
        rotation: Orbit::home(),
        distance: 6.2 / aspect.clamp(0.4, 1.0),
    };
}

fn move_camera(orbit: Res<Orbit>, mut camera: Single<&mut Transform, With<MainCamera>>) {
    let position = orbit.rotation * Vec3::new(0.0, 0.0, orbit.distance);
    // The paper sits a little high: the instructions take the bottom of the window.
    let target = Vec3::new(0.0, -0.55, 0.0);
    **camera = Transform::from_translation(position + target).looking_at(target, orbit.rotation * Vec3::Y);
}
