//! The mouse cursor says what a press does: an open hand over the flap that folds (a little
//! around it too), a closed hand while it folds, two arrows around a circle where a drag turns
//! the model. The panels and buttons keep their own cursors (feathers).
use bevy::asset::RenderAssetUsages;
use bevy::picking::cursor::{DefaultCursor, EntityCursor, OverrideCursor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CustomCursor, CustomCursorImage, SystemCursorIcon};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// What the pointer would do, or does, set by `crate::view`.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    /// Over a panel, or no mouse.
    #[default]
    Free,
    /// A press takes the flap.
    OverFlap,
    Folding,
    /// A press turns the model.
    OverTurn,
    Turning,
}

#[derive(Resource)]
struct TurnCursor(Handle<Image>);

const SIZE: u32 = 32;

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hand>()
            .add_systems(Startup, setup)
            .add_systems(Update, show.after(crate::view::ViewSystems));
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(TurnCursor(images.add(turn_icon())));
}

/// Two arrows chasing each other around a circle, white on a dark outline.
fn turn_icon() -> Image {
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("cursor size");
    let c = SIZE as f32 / 2.0;
    let r = 9.5;
    let at = |deg: f32, r: f32| {
        let a = deg.to_radians();
        (c + r * a.cos(), c + r * a.sin())
    };
    let mut arcs = PathBuilder::new();
    let mut heads = PathBuilder::new();
    for start in [195.0f32, 15.0] {
        // An arc of 120°, a head of 30° after it.
        let (x, y) = at(start, r);
        arcs.move_to(x, y);
        for k in 1..=12 {
            let (x, y) = at(start + 10.0 * k as f32, r);
            arcs.line_to(x, y);
        }
        let (x, y) = at(start + 150.0, r);
        heads.move_to(x, y);
        let (x, y) = at(start + 118.0, r + 5.5);
        heads.line_to(x, y);
        let (x, y) = at(start + 118.0, r - 5.5);
        heads.line_to(x, y);
        heads.close();
    }
    let arcs = arcs.finish().expect("cursor arcs");
    let heads = heads.finish().expect("cursor heads");
    let paint = |rgb: u8| {
        let mut paint = Paint::default();
        paint.set_color_rgba8(rgb, rgb, rgb, 255);
        paint.anti_alias = true;
        paint
    };
    let stroke = |width: f32| Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    let none = Transform::identity();
    for (width, rgb) in [(5.0, 20), (2.5, 255)] {
        pixmap.stroke_path(&arcs, &paint(rgb), &stroke(width), none, None);
        pixmap.stroke_path(&heads, &paint(rgb), &stroke(width - 2.5), none, None);
    }
    pixmap.fill_path(&heads, &paint(255), tiny_skia::FillRule::Winding, none, None);
    // The cursor wants straight alpha.
    let data = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn show(hand: Res<Hand>, turn: Res<TurnCursor>, mut hovering: ResMut<DefaultCursor>, mut held: ResMut<OverrideCursor>) {
    if !hand.is_changed() {
        return;
    }
    let turning = EntityCursor::Custom(CustomCursor::Image(CustomCursorImage {
        handle: turn.0.clone(),
        hotspot: (SIZE as u16 / 2, SIZE as u16 / 2),
        ..default()
    }));
    let system = EntityCursor::System;
    // Hovering, the buttons under the pointer win; held, the cursor stays what it took.
    let (hover, hold) = match *hand {
        Hand::Free => (system(SystemCursorIcon::Default), None),
        Hand::OverFlap => (system(SystemCursorIcon::Grab), None),
        Hand::OverTurn => (turning, None),
        Hand::Folding => (
            system(SystemCursorIcon::Grabbing),
            Some(system(SystemCursorIcon::Grabbing)),
        ),
        Hand::Turning => (turning.clone(), Some(turning)),
    };
    if hovering.0 != hover {
        hovering.0 = hover;
    }
    held.0 = hold;
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_turn_icon_is_drawn() {
        let image = super::turn_icon();
        let data = image.data.expect("pixels");
        let opaque = data.chunks(4).filter(|p| p[3] == 255).count();
        assert!(opaque > 100, "{opaque} opaque pixels");
        if let Ok(path) = std::env::var("BEVYGAMI_DUMP_CURSOR") {
            std::fs::write(path, &data).expect("dump");
        }
    }
}
