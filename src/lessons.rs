//! The lessons: sequences of folds with their instructions, from the first valley fold to the
//! traditional models (the paper cup, the dog face) and the bases of the diagrams (kite, blintz).
//!
//! The points are in paper coordinates, the square `[-1, 1]²` with its colored side up. Most
//! lessons show the square as a diamond (turned by 45°): the corner `BL` is then at the bottom,
//! `TR` at the top, `TL` on the left and `BR` on the right.
use std::f64::consts::{FRAC_PI_4, SQRT_2};

use bevy::math::DVec2;

use crate::fold::{Fold, Line, Paper, Pose};

pub const BL: DVec2 = DVec2::new(-1.0, -1.0);
pub const BR: DVec2 = DVec2::new(1.0, -1.0);
pub const TR: DVec2 = DVec2::new(1.0, 1.0);
pub const TL: DVec2 = DVec2::new(-1.0, 1.0);
pub const CENTER: DVec2 = DVec2::ZERO;

const fn p(x: f64, y: f64) -> DVec2 {
    DVec2::new(x, y)
}

pub struct Step {
    pub fold: Fold,
    pub text: &'static str,
}

pub struct Lesson {
    pub title: &'static str,
    /// What the lesson teaches, under the title.
    pub about: &'static str,
    /// The square shown as a diamond.
    pub diamond: bool,
    pub steps: Vec<Step>,
    /// Said when the last fold is done.
    pub done: &'static str,
}

impl Lesson {
    pub fn start(&self) -> Paper {
        Paper::square(if self.diamond {
            Pose::rotation(FRAC_PI_4)
        } else {
            Pose::IDENTITY
        })
    }

    pub fn folds(&self) -> Vec<Fold> {
        self.steps.iter().map(|s| s.fold).collect()
    }
}

fn step(fold: Fold, text: &'static str) -> Step {
    Step { fold, text }
}

pub fn all() -> Vec<Lesson> {
    vec![
        diagonal_fold(),
        mountain_fold(),
        precreases(),
        kite_base(),
        blintz(),
        pleats(),
        paper_cup(),
        dog_face(),
    ]
}

fn diagonal_fold() -> Lesson {
    Lesson {
        title: "1. The diagonal fold",
        about: "The valley fold: the paper comes toward you, the crease sinks like a valley. Diagrams draw it dashed.",
        diamond: true,
        steps: vec![
            step(
                Fold::bring(BL, TR),
                "Valley fold the bottom corner up to the top corner. Drag the paper, or press Fold.",
            ),
            step(
                Fold::bring(TL, BR),
                "Valley fold in half again: the left corner onto the right corner.",
            ),
        ],
        done: "A triangle, folded twice on the diagonals. The faded lines are the folds you made.",
    }
}

fn mountain_fold() -> Lesson {
    Lesson {
        title: "2. The mountain fold",
        about: "The mountain fold goes away from you, behind the paper. Diagrams draw it dash-dot-dot.",
        diamond: true,
        steps: vec![
            step(
                Fold::mountain(Line::Bring(BL, TR), BL),
                "Mountain fold the bottom corner behind, up to the top corner. Turn the paper to see it go behind.",
            ),
            step(
                Fold::mountain(Line::Bring(TL, BR), TL),
                "Mountain fold in half: the left corner goes behind, onto the right corner.",
            ),
        ],
        done: "The same triangle as lesson 1, seen from the other side: a mountain is a valley turned over.",
    }
}

fn precreases() -> Lesson {
    Lesson {
        title: "3. Fold and unfold",
        about: "Precreases: fold, unfold, and the crease stays to guide the folds after it.",
        diamond: false,
        steps: vec![
            step(
                Fold::bring(BL, TR).and_unfold(),
                "Fold the bottom left corner onto the top right corner, then unfold.",
            ),
            step(Fold::bring(BR, TL).and_unfold(), "Fold and unfold the other diagonal."),
            step(
                Fold::bring(BL, TL).and_unfold(),
                "Fold the bottom edge onto the top edge (a book fold), then unfold.",
            ),
            step(
                Fold::bring(BL, BR),
                "Book fold: the left edge onto the right edge, and keep it folded.",
            ),
        ],
        done: "Three precreases and a book fold: the crease pattern shows them all.",
    }
}

fn kite_base() -> Lesson {
    Lesson {
        title: "4. The kite base",
        about: "Bring an edge onto a crease: the fold bisects the corner. The base of many birds and fish.",
        diamond: true,
        steps: vec![
            step(
                Fold::bring(TL, BR).and_unfold(),
                "Fold the left corner onto the right corner and unfold: a vertical crease.",
            ),
            step(
                Fold::valley(Line::EdgeOnto(BL, TL, BL, TR), TL),
                "Valley fold the lower left edge onto the vertical crease.",
            ),
            step(
                Fold::valley(Line::EdgeOnto(BL, BR, BL, TR), BR),
                "Valley fold the lower right edge onto the vertical crease.",
            ),
        ],
        done: "The kite base.",
    }
}

fn blintz() -> Lesson {
    Lesson {
        title: "5. The blintz",
        about: "Four corners to the center, named after the folded pancake. A base of boxes and fortune tellers.",
        diamond: false,
        steps: vec![
            step(Fold::bring(BL, TR).and_unfold(), "Fold and unfold a diagonal."),
            step(
                Fold::bring(BR, TL).and_unfold(),
                "Fold and unfold the other diagonal: they cross at the center.",
            ),
            step(
                Fold::bring(BL, CENTER),
                "Valley fold the bottom left corner to the center.",
            ),
            step(Fold::bring(BR, CENTER), "The bottom right corner to the center."),
            step(Fold::bring(TR, CENTER), "The top right corner to the center."),
            step(
                Fold::bring(TL, CENTER),
                "And the last corner: the blintz is a smaller square.",
            ),
        ],
        done: "The blintz base, a square half the area of the sheet.",
    }
}

fn pleats() -> Lesson {
    Lesson {
        title: "6. Pleats",
        about: "A pleat is a valley and a mountain side by side; pleats stack in levels, the ground of box pleating.",
        diamond: false,
        steps: vec![
            step(
                Fold::bring(BL, TL).and_unfold(),
                "Fold the bottom edge onto the top edge and unfold: the middle crease.",
            ),
            step(
                Fold::bring(BL, p(-1.0, 0.0)),
                "Valley fold the bottom edge up to the middle crease.",
            ),
            step(
                Fold::bring(BL, p(-1.0, -0.5)).flap(),
                "Fold the edge of that flap back down to its fold: a pleat, two levels.",
            ),
            step(
                Fold::bring(TL, p(-1.0, 0.0)),
                "Valley fold the top edge down to the middle crease.",
            ),
            step(Fold::bring(TL, p(-1.0, 0.5)).flap(), "Pleat it back up to its fold."),
            step(
                Fold::mountain(Line::Bring(BL, BR), BL),
                "Mountain fold the strip in half, left behind right: the pleats stack in four levels.",
            ),
        ],
        done: "Pleats on both sides and a fold on top of them: count the layers on the edge.",
    }
}

/// Where the right corner of the triangle lands on the left edge for a paper cup: the flap's top
/// edge is then level, at `4 - 2√2` of the side from the left corner.
const CUP: f64 = 4.0 - 2.0 * SQRT_2;

fn paper_cup() -> Lesson {
    Lesson {
        title: "7. The paper cup",
        about: "A traditional model: folds of all the layers, then of one flap only.",
        diamond: true,
        steps: vec![
            step(
                Fold::bring(BL, TR),
                "Valley fold the bottom corner up to the top corner: a triangle.",
            ),
            step(
                Fold::bring(BR, p(-1.0 + CUP, 1.0)),
                "Fold the right corner to the left edge, so the top of the flap is level.",
            ),
            step(
                Fold::bring(TL, p(1.0 - CUP, -1.0)),
                "Fold the left corner to the right corner of that flap.",
            ),
            step(
                Fold::valley(Line::Through(p(-1.0 + CUP, 1.0), p(1.0 - CUP, -1.0)), BL).flap(),
                "Valley fold the front flap at the top down over the front.",
            ),
            step(
                Fold::mountain(Line::Through(p(-1.0 + CUP, 1.0), p(1.0 - CUP, -1.0)), TR).flap(),
                "Mountain fold the back flap down behind.",
            ),
        ],
        done: "A paper cup: open it from the top. It holds water for a while.",
    }
}

fn dog_face() -> Lesson {
    Lesson {
        title: "8. The dog face",
        about: "The first model of many folders: a triangle, two ears and a nose.",
        diamond: true,
        steps: vec![
            step(
                Fold::bring(TR, BL),
                "Valley fold the top corner down to the bottom corner: a triangle pointing down.",
            ),
            step(
                Fold::valley(Line::Through(p(-0.4, 0.4), p(0.2, 1.0)), TL),
                "Valley fold the left corner down: the first ear.",
            ),
            step(
                Fold::valley(Line::Through(p(0.4, -0.4), p(1.0, -0.2)), BR),
                "Fold the right corner down the same way: the second ear.",
            ),
            step(
                Fold::valley(Line::Through(p(0.6, 1.0), p(1.0, 0.6)), TR).flap(),
                "Valley fold the tip of the front layer up: the nose.",
            ),
        ],
        done: "Woof. Draw the eyes and the nose with a pen.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold::plan_all;

    #[test]
    fn every_lesson_folds() {
        for lesson in all() {
            let result = plan_all(lesson.start(), &lesson.folds());
            assert!(result.is_ok(), "{}: {:?}", lesson.title, result.err());
            let (plans, last) = result.unwrap();
            let total: f64 = last.faces.iter().map(|f| f.area()).sum();
            assert!(
                (total - 4.0).abs() < 1e-6,
                "{}: the paper area is {total}",
                lesson.title
            );
            for (i, plan) in plans.iter().enumerate() {
                assert!(!plan.chords.is_empty(), "{} step {}: no crease", lesson.title, i + 1);
            }
        }
    }

    #[test]
    fn the_cup_flaps_are_level() {
        let lesson = paper_cup();
        let (plans, _) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        // After the right corner fold, the corner is on the left edge of the triangle.
        let corner = plans[1].after.image_of(BR).unwrap();
        let left = plans[1].after.image_of(TL).unwrap();
        let top = plans[1].after.image_of(TR).unwrap();
        assert!(crate::fold::cross(top - left, corner - left).abs() < 1e-9);
        // The two flap tips are level with each other.
        let other = plans[2].after.image_of(TL).unwrap();
        assert!((other.y - corner.y).abs() < 1e-9);
    }

    #[test]
    fn the_nose_folds_only_the_front() {
        let lesson = dog_face();
        let (plans, _) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        assert_eq!(plans[3].moving.iter().filter(|m| **m).count(), 1);
    }
}
