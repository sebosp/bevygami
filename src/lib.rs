//! Bevygami: learn origami step by step, the paper folds in 3D.
use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;

pub mod cp;
pub mod cpview;
pub mod fold;
pub mod lessons;
pub mod paper;
pub mod session;
pub mod settings;
pub mod theme;
pub mod ui;
pub mod view;

pub struct BevygamiPlugin;

impl Plugin for BevygamiPlugin {
    fn build(&self, app: &mut App) {
        let settings = settings::Settings::load();
        app.insert_resource(session::Session::new(settings.lesson, settings.step))
            .insert_resource(settings.theme)
            .insert_resource(settings)
            .add_plugins((
                FeathersPlugins,
                theme::ThemePlugin,
                session::SessionPlugin,
                paper::PaperPlugin,
                view::ViewPlugin,
                cpview::CpViewPlugin,
                ui::UiPlugin,
            ));
    }
}
