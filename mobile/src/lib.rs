//! Bevygami on Android: full screen, the same plugin as the desktop and the web.
use bevy::prelude::*;
use bevy::window::WindowMode;
use bevygami::BevygamiPlugin;

#[bevy_main]
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevygami".into(),
                resizable: false,
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BevygamiPlugin)
        .run();
}
