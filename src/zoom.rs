//! The zoom of the panels and the text, for a display that is hard to read: Ctrl and the mouse
//! wheel, a pinch of the trackpad, Ctrl+= Ctrl+- Ctrl+0, or the buttons of the settings. It is
//! saved with the settings. In the browser, its own zoom takes Ctrl and the wheel. The paper has
//! its own zoom, the wheel alone, see [`crate::view`].
use bevy::input::ButtonState;
use bevy::input::gestures::PinchGesture;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::settings::Settings;

pub const MIN_ZOOM: f32 = 0.6;
pub const MAX_ZOOM: f32 = 3.0;
/// The zoom of a key press or a button, a factor.
const STEP: f32 = 1.1;

/// Changes the zoom.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub enum Zoom {
    /// Multiplies it.
    By(f32),
    Reset,
}

impl Zoom {
    pub const IN: Zoom = Zoom::By(STEP);
    pub const OUT: Zoom = Zoom::By(1.0 / STEP);

    /// The zoom of a key pressed with Ctrl, as in the browsers.
    pub fn of_key(key: &Key) -> Option<Zoom> {
        let Key::Character(c) = key else {
            return None;
        };
        match c.as_str() {
            "=" | "+" => Some(Zoom::IN),
            "-" | "_" => Some(Zoom::OUT),
            "0" => Some(Zoom::Reset),
            _ => None,
        }
    }

    /// The new zoom, between the bounds.
    pub fn apply(self, zoom: f32) -> f32 {
        match self {
            Zoom::By(by) => (zoom * by).clamp(MIN_ZOOM, MAX_ZOOM),
            Zoom::Reset => 1.0,
        }
    }
}

pub struct ZoomPlugin;

impl Plugin for ZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Zoom>()
            .add_systems(Update, (gestures, apply_zoom).chain());
    }
}

/// Ctrl and the wheel, Ctrl and a key, or a pinch.
fn gestures(
    mut wheel: MessageReader<MouseWheel>,
    mut pinch: MessageReader<PinchGesture>,
    mut keyboard: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut zoom: MessageWriter<Zoom>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    for event in wheel.read() {
        if !ctrl || cfg!(target_arch = "wasm32") {
            continue;
        }
        let notches = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 40.0,
        };
        if notches != 0.0 {
            zoom.write(Zoom::By(STEP.powf(notches.clamp(-3.0, 3.0))));
        }
    }
    for PinchGesture(delta) in pinch.read() {
        zoom.write(Zoom::By((1.0 + delta).clamp(0.5, 2.0)));
    }
    for key in keyboard.read() {
        if ctrl
            && key.state == ButtonState::Pressed
            && let Some(change) = Zoom::of_key(&key.logical_key)
        {
            zoom.write(change);
        }
    }
}

fn apply_zoom(mut changes: MessageReader<Zoom>, mut settings: ResMut<Settings>, mut scale: ResMut<UiScale>) {
    let mut zoom = settings.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    let mut changed = false;
    for change in changes.read() {
        zoom = change.apply(zoom);
        changed = true;
    }
    if changed && zoom != settings.zoom {
        settings.zoom = zoom;
        settings.save();
    }
    if scale.0 != zoom {
        scale.0 = zoom;
    }
}

/// The zoom in percent, for the settings.
pub fn percent(zoom: f32) -> String {
    format!("{:.0}%", zoom * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range() {
        let mut zoom = 1.0;
        for _ in 0..50 {
            zoom = Zoom::IN.apply(zoom);
        }
        assert_eq!(zoom, MAX_ZOOM);
        assert_eq!(Zoom::Reset.apply(zoom), 1.0);
        assert!((Zoom::OUT.apply(Zoom::IN.apply(1.0)) - 1.0).abs() < 1e-6);
        assert_eq!(Zoom::of_key(&Key::Character("=".into())), Some(Zoom::IN));
        assert_eq!(Zoom::of_key(&Key::Character("a".into())), None);
    }
}
