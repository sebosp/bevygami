// No console window next to the game on Windows, in the release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use bevy::prelude::*;
use bevygami::BevygamiPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevygami".into(),
            // The app id of Wayland and the class of X11, as the desktop file.
            name: Some("bevygami".into()),
            resolution: (1280, 860).into(),
            // The web build fills the page, see index.html.
            canvas: Some("#bevygami".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(BevygamiPlugin);
    #[cfg(not(target_arch = "wasm32"))]
    snapshot::add(&mut app);
    app.run();
}

/// `BEVYGAMI_SCREENSHOT=out.png` saves a screenshot and exits, to check the screen without a person
/// at the mouse. `BEVYGAMI_LESSON=n` (from 1) and `BEVYGAMI_STEP=n` pick the step,
/// `BEVYGAMI_T=0.5` stops the fold half way, `BEVYGAMI_YAW` / `BEVYGAMI_PITCH` turn the view
/// (radians), `BEVYGAMI_PANEL=settings|help` opens a panel, `BEVYGAMI_THEME=light`.
#[cfg(not(target_arch = "wasm32"))]
mod snapshot {
    use bevy::prelude::*;
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    use bevy::window::PrimaryWindow;
    use bevygami::session::Session;
    use bevygami::settings::Settings;
    use bevygami::theme::ThemeMode;
    use bevygami::ui::Panel;
    use bevygami::view::Orbit;

    #[derive(Resource)]
    struct Snapshot(String);

    fn var<T: std::str::FromStr>(name: &str) -> Option<T> {
        std::env::var(name).ok().and_then(|v| v.parse().ok())
    }

    pub fn add(app: &mut App) {
        let Ok(path) = std::env::var("BEVYGAMI_SCREENSHOT") else {
            return;
        };
        app.insert_resource(Snapshot(path))
            .add_systems(Startup, no_vsync)
            .add_systems(Update, run);
    }

    /// A window in the background is throttled by the compositor with vsync.
    fn no_vsync(mut window: Single<&mut Window, With<PrimaryWindow>>) {
        window.present_mode = bevy::window::PresentMode::AutoNoVsync;
    }

    #[allow(clippy::too_many_arguments)]
    fn run(
        mut frame: Local<u32>,
        mut shot_at: Local<Option<u32>>,
        time: Res<Time<Real>>,
        snapshot: Res<Snapshot>,
        mut session: ResMut<Session>,
        mut settings: ResMut<Settings>,
        mut theme: ResMut<ThemeMode>,
        mut panel: ResMut<Panel>,
        mut orbit: ResMut<Orbit>,
        mut commands: Commands,
        mut exit: MessageWriter<AppExit>,
    ) {
        *frame += 1;
        if *frame == 5 {
            if let Some(lesson) = var::<usize>("BEVYGAMI_LESSON") {
                session.load(lesson.saturating_sub(1), var("BEVYGAMI_STEP").unwrap_or(0));
            }
            if std::env::var("BEVYGAMI_THEME").is_ok_and(|t| t == "light") {
                *theme = ThemeMode::Light;
                settings.theme = ThemeMode::Light;
            }
            *panel = match std::env::var("BEVYGAMI_PANEL").as_deref() {
                Ok("settings") => Panel::Settings,
                Ok("help") => Panel::Help,
                _ => Panel::None,
            };
        }
        if *frame == 8 {
            let yaw: f32 = var("BEVYGAMI_YAW").unwrap_or(0.0);
            let pitch: f32 = var("BEVYGAMI_PITCH").unwrap_or(0.0);
            orbit.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * Orbit::home();
        }
        if *frame >= 8
            && let Some(t) = var::<f32>("BEVYGAMI_T")
        {
            session.t = t;
        }
        // The render pipelines compile in the background, a while after the first frames.
        let wait: f32 = var("BEVYGAMI_WAIT").unwrap_or(3.0);
        if time.elapsed_secs() > wait && shot_at.is_none() {
            *shot_at = Some(*frame);
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(snapshot.0.clone()));
        }
        if shot_at.is_some_and(|f| *frame > f + 20) {
            exit.write(AppExit::Success);
        }
    }
}
