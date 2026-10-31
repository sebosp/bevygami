//! Bevygami: learn origami step by step, the paper folds in 3D.
use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;

pub mod cp;
pub mod cpview;
pub mod cursor;
pub mod fold;
pub mod layers;
pub mod lessons;
pub mod motion;
pub mod paper;
pub mod session;
pub mod settings;
pub mod theme;
pub mod ui;
pub mod view;
pub mod zoom;

pub struct BevygamiPlugin;

impl Plugin for BevygamiPlugin {
    fn build(&self, app: &mut App) {
        let settings = settings::Settings::load();
        // Back where the player left, the finished model included.
        let step = settings.steps.get(&settings.lesson).copied().unwrap_or(settings.step);
        app.insert_resource(session::Session::new(settings.lesson, step))
            .insert_resource(settings.theme)
            .insert_resource(settings)
            .add_plugins((
                FeathersPlugins,
                theme::ThemePlugin,
                session::SessionPlugin,
                paper::PaperPlugin,
                view::ViewPlugin,
                cursor::CursorPlugin,
                cpview::CpViewPlugin,
                ui::UiPlugin,
                zoom::ZoomPlugin,
            ));
    }
}
