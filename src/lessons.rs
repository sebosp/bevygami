//! The lessons: sequences of folds with their instructions, from the first valley fold to the
//! traditional models (the paper cup, the dog face) and the bases of the diagrams (kite, blintz).
//!
//! The points are in paper coordinates, the square `[-1, 1]²` with its colored side up. Most
//! lessons show the square as a diamond (turned by 45°): the corner `BL` is then at the bottom,
//! `TR` at the top, `TL` on the left and `BR` on the right.
use std::f64::consts::{FRAC_PI_4, SQRT_2};

use bevy::math::{DMat2, DVec2};

use crate::fold::{Collapse, Fold, FoldKind, Grip, Line, Paper, Pose};

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
    /// The colored side up at the start; the traditional models start white side up, so the
    /// color ends outside.
    pub colored_up: bool,
    pub steps: Vec<Step>,
    /// Said when the last fold is done.
    pub done: &'static str,
}

impl Lesson {
    pub fn start(&self) -> Paper {
        Paper::square(self.start_pose())
    }

    /// The sheet turned as a diamond, and over (mirrored by `x ↦ -x`) when the white side is up.
    pub fn start_pose(&self) -> Pose {
        let turn = Pose::rotation(if self.diamond { FRAC_PI_4 } else { 0.0 });
        if self.colored_up {
            turn
        } else {
            Pose {
                m: turn.m * DMat2::from_diagonal(DVec2::new(-1.0, 1.0)),
                t: DVec2::ZERO,
            }
        }
    }

    /// The folds on the sheet as it lies: mirrored with it, they look the same.
    pub fn folds(&self) -> Vec<Fold> {
        self.steps
            .iter()
            .map(|s| if self.colored_up { s.fold } else { s.fold.mirrored() })
            .collect()
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
        waterbomb_base(),
        preliminary_base(),
        bird_base(),
        crane(),
    ]
}

fn diagonal_fold() -> Lesson {
    Lesson {
        title: "1. The diagonal fold",
        about: "The valley fold: the paper comes toward you, the crease sinks like a valley. Diagrams draw it dashed.",
        diamond: true,
        colored_up: true,
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
        colored_up: true,
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
        colored_up: true,
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
        colored_up: true,
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
        colored_up: true,
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
        colored_up: true,
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
        about: "A traditional model, started white side up so the color ends outside: folds of all the layers, then of one flap only.",
        diamond: true,
        colored_up: false,
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
        about: "The first model of many folders, white side up at the start: a triangle, two ears and a nose.",
        diamond: true,
        colored_up: false,
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

fn waterbomb_base() -> Lesson {
    Lesson {
        title: "9. The waterbomb base",
        about: "The inside reverse fold: a corner pushed in between its own layers, the crease along its spine turns inside out.",
        diamond: false,
        colored_up: true,
        steps: vec![
            step(
                Fold::bring(TL, BL),
                "Valley fold the top edge down onto the bottom edge.",
            ),
            step(
                Fold::inside_reverse(Line::Through(CENTER, BL), p(-1.0, 0.0)),
                "Inside reverse fold the left corner: open the layers a little and push the corner in between them, along the diagonal.",
            ),
            step(
                Fold::inside_reverse(Line::Through(CENTER, BR), p(1.0, 0.0)),
                "Inside reverse fold the right corner the same way.",
            ),
        ],
        done: "The waterbomb base: a triangle with two flaps on each side. The waterbomb and the jumping frog start here.",
    }
}

/// The steps of the preliminary (square) base, white side up: from a triangle, both corners
/// reversed inside along the book fold lines.
fn preliminary_steps() -> Vec<Step> {
    vec![
        step(
            Fold::bring(TR, BL),
            "Valley fold the top corner down to the bottom corner: a triangle.",
        ),
        step(
            Fold::inside_reverse(Line::Through(CENTER, p(-1.0, 0.0)), TL),
            "Inside reverse fold the left corner down to the bottom, between the layers.",
        ),
        step(
            Fold::inside_reverse(Line::Through(CENTER, p(0.0, -1.0)), BR),
            "Inside reverse fold the right corner the same way: a square standing on a corner.",
        ),
    ]
}

fn preliminary_base() -> Lesson {
    Lesson {
        title: "10. The preliminary base",
        about: "Also called the square base, the start of the crane, the lily and the frog. The waterbomb base turned inside out.",
        diamond: true,
        colored_up: false,
        steps: preliminary_steps(),
        done: "The preliminary base: closed at the top, four flaps at the open bottom. The bird base and the crane come next, with the petal fold.",
    }
}

/// Where the kite creases of the bird base meet the book folds: `2 - √2` from the center.
const K: f64 = 2.0 - SQRT_2;

/// The petal fold of the front flap of the preliminary base: the kite creases from the corners of
/// the front quadrant (`TR`) to the book folds, and from the corners of the side quadrants on
/// them, the hinge of the petal, and the book folds opened flat between the kite creases and the
/// edges. At a kite tip four creases meet: 112.5° + 67.5° = 180°, Kawasaki holds; the book fold is
/// a mountain from the colored side, the kites and the hinge valleys, Maekawa holds (3 V, 1 M).
const BIRD_FRONT: Collapse = Collapse {
    fold: &[
        (TR, p(K, 0.0), FoldKind::Valley),
        (TR, p(0.0, K), FoldKind::Valley),
        (TL, p(0.0, K), FoldKind::Valley),
        (BR, p(K, 0.0), FoldKind::Valley),
        (p(K, 0.0), p(0.0, K), FoldKind::Valley),
    ],
    flat: &[(p(K, 0.0), p(1.0, 0.0)), (p(0.0, K), p(0.0, 1.0))],
    mirror: false,
};

/// The same on the back quadrant (`BL`).
const BIRD_BACK: Collapse = Collapse {
    fold: &[
        (BL, p(-K, 0.0), FoldKind::Valley),
        (BL, p(0.0, -K), FoldKind::Valley),
        (TL, p(-K, 0.0), FoldKind::Valley),
        (BR, p(0.0, -K), FoldKind::Valley),
        (p(-K, 0.0), p(0.0, -K), FoldKind::Valley),
    ],
    flat: &[(p(-K, 0.0), p(-1.0, 0.0)), (p(0.0, -K), p(0.0, -1.0))],
    mirror: false,
};

fn bird_base() -> Lesson {
    let mut steps = preliminary_steps();
    let bottom = DVec2::new(0.0, -SQRT_2);
    let side = SQRT_2 - 1.0;
    let precreases = |which: &'static str| {
        vec![
            step(
                Fold::valley(
                    Line::Folded(bottom, DVec2::new(-side, -side)),
                    Grip::Top(DVec2::new(-0.55, -0.75)),
                )
                .flap()
                .and_unfold(),
                if which == "front" {
                    "Fold the lower left edge of the front flap to the center line, and unfold."
                } else {
                    "Same on this side: the lower left edge to the center, and unfold."
                },
            ),
            step(
                Fold::valley(
                    Line::Folded(bottom, DVec2::new(side, -side)),
                    Grip::Top(DVec2::new(0.55, -0.75)),
                )
                .flap()
                .and_unfold(),
                "The lower right edge to the center line, and unfold.",
            ),
            step(
                Fold::valley(
                    Line::Folded(DVec2::new(-1.0, -side), DVec2::new(1.0, -side)),
                    Grip::Top(DVec2::new(0.0, -0.15)),
                )
                .flap()
                .and_unfold(),
                "Fold the top triangle down along the ends of those creases, and unfold.",
            ),
        ]
    };
    steps.extend(precreases("front"));
    steps.push(step(Fold::turn_over(), "Turn the model over."));
    steps.extend(precreases("back"));
    steps.push(step(Fold::turn_over(), "Turn it back over."));
    steps.push(step(
        Fold::collapse(BIRD_FRONT, p(-0.75, 0.8)),
        "Petal fold: lift the bottom corner of the front layer up along the top crease, the sides come in along the kite creases.",
    ));
    steps.push(step(Fold::turn_over(), "Turn the model over."));
    steps.push(step(
        Fold::collapse(BIRD_BACK, p(-0.75, 0.8)),
        "The petal fold on this side too.",
    ));
    steps.push(step(Fold::turn_over(), "Turn it back over."));
    Lesson {
        title: "11. The bird base",
        about: "Kite creases and petal folds on both sides of the preliminary base: the base of the crane, the flapping bird and many more.",
        diamond: true,
        colored_up: false,
        steps,
        done: "The bird base: two long petals up, two flaps down that become the neck and the tail of the crane.",
    }
}

/// The corner at the tip of the back petal of the bird base.
const BACK_WING: DVec2 = BL;

/// The creases of a leg narrowing of the crane, around the kite tip `(0, K)`: the front of the
/// left leg. The line that brings the lower edge of the leg onto the center line bisects the 22.5°
/// at the foot; it goes through the front layer of the leg and the side folded inside it, and at
/// the hinge line, where that side joins the side of the petal flat, on into the corner of the
/// petal, through every layer down to the inside of the leg: a closed loop of six creases from
/// the corner `TL` of the sheet back to it, crossing the kite, the hinge and the book fold, the
/// paper inside it folds over in one piece. In the diagrams the corner of the petal tucks in, the
/// leg is narrow.
fn narrowing() -> Vec<(DVec2, DVec2, FoldKind)> {
    // Half of the 22.5° of the leg at its foot.
    let t = (11.25f64).to_radians();
    // Where the ray from `from` along `d` meets the line through `a` and `b`.
    let hit = |from: DVec2, d: DVec2, a: DVec2, b: DVec2| {
        let e = b - a;
        from + d * (crate::fold::cross(e, a - from) / crate::fold::cross(e, d))
    };
    // Across a folded crease the line goes on mirrored along it.
    let bounce = |d: DVec2, a: DVec2, b: DVec2| {
        let e = (b - a).normalize();
        d - e * (2.0 * d.dot(e))
    };
    let kite_tip = p(0.0, K);
    let d0 = DVec2::new(t.cos(), -t.sin());
    // The side inside the leg, then flat on into the side of the petal.
    let a = hit(TL, d0, CENTER, p(0.0, 1.0));
    let c = hit(a, d0, kite_tip, TR);
    // The petal, the middle of the front flap below it, the leg above the hinge line.
    let d2 = bounce(d0, kite_tip, TR);
    let d = hit(c, d2, p(K, 0.0), kite_tip);
    let d3 = bounce(d2, p(K, 0.0), kite_tip);
    let e = hit(d, d3, CENTER, kite_tip);
    let d4 = bounce(d3, CENTER, kite_tip);
    let b = hit(e, d4, kite_tip, p(-1.0, K - 1.0));
    use FoldKind::{Mountain as M, Valley as V};
    vec![(TL, a, V), (a, c, V), (c, d, M), (d, e, V), (e, b, M), (b, TL, M)]
}

/// The four leg narrowings, the front of the left leg turned onto the others: the front of the
/// right leg mirrored across the diagonal `y = x`, the back of the left leg across `y = -x`, the
/// back of the right leg turned by a half turn.
static NARROWINGS: std::sync::LazyLock<[Vec<(DVec2, DVec2, FoldKind)>; 4]> = std::sync::LazyLock::new(|| {
    let left = narrowing();
    let turned = |f: fn(DVec2) -> DVec2| left.iter().map(|(a, b, k)| (f(*a), f(*b), *k)).collect();
    [
        left.clone(),
        turned(|v| DVec2::new(v.y, v.x)),
        turned(|v| DVec2::new(-v.y, -v.x)),
        turned(|v| -v),
    ]
});

fn crane() -> Lesson {
    let mut steps = bird_base().steps;
    let narrow = |i: usize| {
        Fold::collapse(
            Collapse {
                fold: &NARROWINGS[i],
                flat: &[],
                mirror: false,
            },
            // A point on the back petal, it does not move.
            p(-0.9, -0.9),
        )
    };
    steps.push(step(
        narrow(0),
        "Narrow the legs: valley fold the lower left edge of the front layer to the center line. The corner of the petal above folds over with it.",
    ));
    steps.push(step(narrow(1), "And the lower right edge."));
    steps.push(step(Fold::turn_over(), "Turn the model over."));
    // Turned over, the back of the right leg is on the left.
    steps.push(step(
        narrow(3),
        "The same on this side: the lower left edge to the center line.",
    ));
    steps.push(step(narrow(2), "And the lower right edge."));
    steps.push(step(Fold::turn_over(), "Turn it back over."));
    // The legs go up between the wings, at 30° from the vertical. The reverse fold stays below the
    // hinge line of the petals (`y = 1 - √2`), in the leg alone, and high enough for the neck and
    // the tail to come out above the wings.
    let hinge = DVec2::new(0.0, -0.45);
    let up = |x: f64| {
        let a = (195.0f64).to_radians();
        Line::Folded(hinge, hinge + DVec2::new(x * a.cos(), a.sin()))
    };
    steps.push(step(
        Fold::inside_reverse(up(1.0), TL),
        "Inside reverse fold the left leg up between the wings: the neck.",
    ));
    steps.push(step(
        Fold::inside_reverse(up(-1.0), BR),
        "Inside reverse fold the right leg up: the tail.",
    ));
    // The head: the tip of the neck reversed down. The neck goes up at 120° from the hinge, as long
    // as the leg was below it (the foot of the leg is at `y = -√2`).
    let along = DVec2::new(-0.5, 0.75f64.sqrt());
    let tip = hinge + along * (hinge.y + SQRT_2);
    let at = tip - along * 0.2;
    let head = (165.0f64).to_radians();
    steps.push(step(
        Fold::inside_reverse(Line::Folded(at, at + DVec2::new(head.cos(), head.sin())), TL),
        "Inside reverse fold the tip of the neck down: the head.",
    ));
    // The wings down along the top of the body, the center of the sheet: lower, the line would go
    // through the top of the body, inside the wings, and the corners of the petals folded with the
    // legs.
    let wings = Line::Folded(DVec2::new(-1.0, 0.0), DVec2::new(1.0, 0.0));
    steps.push(step(
        Fold::valley(wings, Grip::Top(DVec2::new(0.0, 0.45))).flap(),
        "Fold the front wing down.",
    ));
    steps.push(step(
        Fold::mountain(wings, Grip::Paper(BACK_WING)).flap(),
        "Fold the back wing down behind.",
    ));
    Lesson {
        title: "12. The crane",
        about: "The orizuru, the traditional crane: the bird base, narrowed legs reversed into a neck and a tail, a head, the wings down.",
        diamond: true,
        colored_up: false,
        steps,
        done: "A crane. Pull the wings apart gently and the body puffs up. Fold a thousand and make a wish.",
    }
}

/// The creases that are folded on the paper (the faces on both sides are not placed alike), for
/// the flat-foldability checks.
#[cfg(test)]
fn folded_creases(paper: &Paper) -> Vec<crate::fold::Crease> {
    let mut folded = Vec::new();
    for (i, f) in paper.faces.iter().enumerate() {
        for g in &paper.faces[i + 1..] {
            if f.pose.close_to(&g.pose) {
                continue;
            }
            for (a, b) in crate::fold::shared_edges(f, g) {
                if let Some(c) = paper.creases.iter().find(|c| {
                    let near = |x: DVec2| {
                        let ab = c.b - c.a;
                        let t = ((x - c.a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                        x.distance(c.a + ab * t) < 1e-6
                    };
                    near(a) && near(b)
                }) {
                    folded.push(crate::fold::Crease { a, b, ..*c });
                }
            }
        }
    }
    folded
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
            for (i, plan) in plans
                .iter()
                .enumerate()
                .filter(|(_, p)| p.fold.style != crate::fold::Style::TurnOver)
            {
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
    fn the_bases_have_their_shape() {
        // The waterbomb base: a triangle, apex at the middle of the top edge.
        let lesson = waterbomb_base();
        let (_, last) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        let (min, max) = last.bounds();
        assert!(min.distance(DVec2::new(-1.0, -1.0)) < 1e-9 && max.distance(DVec2::new(1.0, 0.0)) < 1e-9);
        assert!(last.creases.iter().filter(|c| c.step >= 1).count() >= 4);
        // Maekawa and Kawasaki hold at the center, as for the base folded by hand.
        let vertices = crate::cp::inner_vertices(&last.creases);
        assert_eq!(vertices.len(), 1);
        assert!(vertices[0].flat_foldable(), "{:?}", vertices[0]);
        // The preliminary base: a square of half the side, standing on a corner, every corner of
        // the sheet at the bottom.
        let lesson = preliminary_base();
        let (_, last) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        let (min, max) = last.bounds();
        assert!((max.x - min.x - SQRT_2).abs() < 1e-9 && (max.y - min.y - SQRT_2).abs() < 1e-9);
        for vertex in crate::cp::inner_vertices(&last.creases) {
            assert!(vertex.flat_foldable(), "{vertex:?}");
        }
        for corner in [BL, BR, TL, TR] {
            let at = last.image_of(DVec2::new(-corner.x, corner.y)).unwrap();
            assert!(at.distance(DVec2::new(0.0, -SQRT_2)) < 1e-9, "{corner} is at {at}");
        }
    }

    #[test]
    fn the_bird_base_collapses() {
        let lesson = bird_base();
        let (_, last) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        let (min, max) = last.bounds();
        assert!((max.x - (SQRT_2 - 1.0)).abs() < 1e-9 && (min.x + SQRT_2 - 1.0).abs() < 1e-9);
        assert!((max.y - (2.0 - SQRT_2)).abs() < 1e-9 && (min.y + SQRT_2).abs() < 1e-9);
        // Two corners at the tip of the petals, two at the bottom.
        let up = [BL, BR, TL, TR]
            .iter()
            .filter(|c| last.image_of(DVec2::new(-c.x, c.y)).unwrap().y > 0.5)
            .count();
        assert_eq!(up, 2);
        // Flat-foldable at every vertex inside the sheet, the creases that are folded.
        for vertex in crate::cp::inner_vertices(&super::folded_creases(&last)) {
            assert!(vertex.flat_foldable(), "{vertex:?}");
        }
    }

    #[test]
    fn the_crane_has_a_neck_a_tail_and_wings_down() {
        let lesson = crane();
        let (_, last) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        let at = |c: DVec2| last.image_of(DVec2::new(-c.x, c.y)).unwrap();
        // The tail up on the right, the head hooked down on the left of the neck.
        let (tail, head) = (at(BR), at(TL));
        assert!(tail.x > 0.35 && tail.y > 0.0, "tail at {tail}");
        assert!(head.x < -0.4 && head.y < tail.y, "head at {head}");
        // Both wing tips down, together, below the body.
        assert!(
            at(BL).distance(at(TR)) < 1e-9 && at(BL).y < -0.5,
            "wings at {} and {}",
            at(BL),
            at(TR)
        );
        let total: f64 = last.faces.iter().map(|f| f.area()).sum();
        assert!((total - 4.0).abs() < 1e-6);
        for vertex in crate::cp::inner_vertices(&super::folded_creases(&last)) {
            assert!(vertex.flat_foldable(), "{vertex:?}");
        }
    }

    #[test]
    fn the_nose_folds_only_the_front() {
        let lesson = dog_face();
        let (plans, _) = plan_all(lesson.start(), &lesson.folds()).unwrap();
        assert_eq!(plans[3].moving.iter().filter(|m| **m).count(), 1);
    }
}
