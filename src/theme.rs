//! The dark and light themes: the feathers themes, the colors of the scene and of the creases.
//!
//! Feathers 0.20 only ships a dark theme, the light one overrides its semantic colors.
//!
//! The creases follow the crease pattern convention of ORIPA and Oriedita, valley blue and
//! mountain red, and the diagram convention of Yoshizawa and Randlett for the line, dashed for a
//! valley and dash-dot-dot for a mountain: the color is never the only cue. The pairs were checked
//! with the dataviz palette validator: on every paper color and on the light surface (`#1f63c7`,
//! `#b8262e`, contrast ≥ 3:1, protan ΔE 22), on the dark surface (`#3f86e0`, `#e05252`, protan
//! ΔE 21).
use bevy::color::Luminance;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::{InheritableThemeTextColor, SurfaceLevel, ThemeProps, UiTheme};
use bevy::feathers::tokens::{self as feathers_tokens, semantic};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::fold::FoldKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Resource)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 2] = [ThemeMode::Dark, ThemeMode::Light];

    pub fn name(self) -> &'static str {
        match self {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
        }
    }
}

pub fn hex(s: &str) -> Color {
    Srgba::hex(s).map(Color::from).unwrap_or(Color::BLACK)
}

/// The creases drawn on the paper, whatever the theme: validated against the paper colors.
pub fn crease_on_paper(kind: FoldKind) -> Color {
    match kind {
        FoldKind::Valley => hex("#1f63c7"),
        FoldKind::Mountain => hex("#b8262e"),
    }
}

/// The colors of the scene around the paper.
#[derive(Debug, Clone, Copy)]
pub struct Scene {
    pub background: Color,
    /// The arrows and the landing outline.
    pub guide: Color,
    pub accent: Color,
    pub dim: Color,
}

pub fn scene(mode: ThemeMode) -> Scene {
    match mode {
        ThemeMode::Dark => Scene {
            background: hex("#1b1c22"),
            guide: hex("#e9ebf2"),
            accent: hex("#3f86e0"),
            dim: hex("#8a8fa3"),
        },
        ThemeMode::Light => Scene {
            background: hex("#e9eaef"),
            guide: hex("#23262f"),
            accent: hex("#1f63c7"),
            dim: hex("#5d6375"),
        },
    }
}

pub fn props(mode: ThemeMode) -> ThemeProps {
    match mode {
        ThemeMode::Dark => create_dark_theme(),
        ThemeMode::Light => light_theme(),
    }
}

fn light_theme() -> ThemeProps {
    let mut props = create_dark_theme();
    let window = hex("#f4f5f8");
    let pane = hex("#e8eaf0");
    let item = hex("#dde0e8");
    let item_active = hex("#cfd3de");
    let accent = hex("#1f63c7");
    let base = [
        (semantic::SURFACE_WINDOW, window),
        (semantic::SURFACE_PANE_BODY, pane),
        (semantic::SURFACE_PANE_HEADER, window),
        (semantic::SURFACE_SUBPANE_BODY, pane),
        (semantic::SURFACE_SUBPANE_HEADER, item),
        (semantic::SURFACE_GROUP, item),
        (semantic::SURFACE_DIALOG, pane),
        (semantic::SURFACE_MENU, window),
        (semantic::BORDER_DEFAULT, hex("#b8bdca")),
        (semantic::TEXT_DEFAULT, hex("#23262f")),
        (semantic::TEXT_DIM, hex("#5d6375")),
        (semantic::TEXT_DISABLED, hex("#23262f").with_alpha(0.4)),
        (semantic::TEXT_ON_ACCENT, Color::WHITE),
        (semantic::FILL_ACCENT_DEFAULT, accent),
        (semantic::FILL_ACCENT_HOVER, accent.lighter(0.05)),
        (semantic::FILL_ACCENT_PRESSED, accent.lighter(0.1)),
        (semantic::FILL_ACCENT_DISABLED, item_active.with_alpha(0.7)),
        (semantic::FILL_SOLID_DEFAULT, item_active),
        (semantic::FILL_SOLID_HOVER, item_active.darker(0.04)),
        (semantic::FILL_SOLID_PRESSED, item_active.darker(0.08)),
        (semantic::FILL_SOLID_DISABLED, item.with_alpha(0.7)),
        (semantic::FILL_ITEM_HOVER, pane.darker(0.04)),
        (semantic::FILL_ITEM_PRESSED, pane.darker(0.08)),
        (semantic::FILL_FIELD_DEFAULT, Color::WHITE),
        (semantic::FILL_FIELD_HOVER, Color::WHITE.darker(0.02)),
        (semantic::FILL_FIELD_PRESSED, Color::WHITE.darker(0.04)),
        (semantic::FOCUS_RING, accent.with_alpha(0.5)),
    ];
    props.semantic_base.extend(base);
    // The dark theme uses white text on its gray controls, dark text reads better on light ones.
    for token in [
        feathers_tokens::BUTTON_TEXT,
        feathers_tokens::SLIDER_TEXT,
        feathers_tokens::MENUITEM_TEXT,
        feathers_tokens::TEXT_INPUT_TEXT,
        feathers_tokens::LISTROW_TEXT,
    ] {
        props.token_assignments.insert(token, semantic::TEXT_DEFAULT);
    }
    // The dark overrides lighten the controls on the higher surfaces, darken them here.
    props.semantic_overrides = HashMap::from([(
        SurfaceLevel::Highest,
        HashMap::from([
            (semantic::FILL_SOLID_DEFAULT, item_active.darker(0.04)),
            (semantic::FILL_FIELD_DEFAULT, Color::WHITE),
            (semantic::FILL_ITEM_HOVER, item.darker(0.04)),
        ]),
    )]);
    props
}

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ThemeMode>()
            .add_systems(PreUpdate, apply_theme_mode.run_if(resource_changed::<ThemeMode>));
    }
}

/// Swaps the [`UiTheme`], feathers updates the backgrounds, the borders and the direct text
/// colors by itself, the inherited text colors are refreshed by inserting them again.
fn apply_theme_mode(
    mode: Res<ThemeMode>,
    mut theme: ResMut<UiTheme>,
    mut clear_color: ResMut<ClearColor>,
    inherited: Query<(Entity, &InheritableThemeTextColor)>,
    mut commands: Commands,
) {
    theme.0 = props(*mode);
    clear_color.0 = scene(*mode).background;
    for (entity, color) in &inherited {
        commands.entity(entity).insert(color.clone());
    }
}
