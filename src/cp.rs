//! Crease patterns: the creases of the unfolded sheet, and the local flat-foldability checks of
//! the CP editors (ORIPA, Oriedita, Box Pleating Studio) and of Lang's *Origami Design Secrets*:
//! at every inner vertex, Maekawa (`|M - V| = 2`) and Kawasaki (the alternate angles sum to π).
//! The folds of the lessons always pass them; the checks are the ground for the crease pattern
//! lessons (box pleating) to come, where the player draws the creases.
use std::f64::consts::PI;

use bevy::math::DVec2;

use crate::fold::{Crease, FoldKind};

/// A vertex of the pattern inside the sheet, with the creases that leave it.
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub at: DVec2,
    /// The direction angle of each crease leaving the vertex, sorted, with its kind.
    pub creases: Vec<(f64, FoldKind)>,
}

impl Vertex {
    pub fn maekawa(&self) -> bool {
        let m = self.creases.iter().filter(|(_, k)| *k == FoldKind::Mountain).count() as i64;
        let v = self.creases.len() as i64 - m;
        (m - v).abs() == 2
    }

    pub fn kawasaki(&self) -> bool {
        let n = self.creases.len();
        if n % 2 == 1 || n < 2 {
            return false;
        }
        let mut alternate = 0.0;
        for i in 0..n {
            let a = self.creases[i].0;
            let b = if i + 1 < n {
                self.creases[i + 1].0
            } else {
                self.creases[0].0 + 2.0 * PI
            };
            if i % 2 == 0 {
                alternate += b - a;
            }
        }
        (alternate - PI).abs() < 1e-6
    }

    pub fn flat_foldable(&self) -> bool {
        self.maekawa() && self.kawasaki()
    }
}

/// The creases split where they cross and meet, the vertices strictly inside the square.
pub fn inner_vertices(creases: &[Crease]) -> Vec<Vertex> {
    let mut points: Vec<DVec2> = Vec::new();
    let add = |p: DVec2, points: &mut Vec<DVec2>| {
        if p.x.abs() < 1.0 - 1e-6 && p.y.abs() < 1.0 - 1e-6 && !points.iter().any(|q| q.distance(p) < 1e-6) {
            points.push(p);
        }
    };
    for (i, c) in creases.iter().enumerate() {
        add(c.a, &mut points);
        add(c.b, &mut points);
        for d in &creases[i + 1..] {
            if let Some(p) = intersection(c.a, c.b, d.a, d.b) {
                add(p, &mut points);
            }
        }
    }
    points
        .into_iter()
        .map(|at| {
            let mut leaving = Vec::new();
            for c in creases {
                for to in [c.a, c.b] {
                    // The crease passes through or ends at the vertex, toward `to`.
                    let on = distance_to_segment(at, c.a, c.b) < 1e-6;
                    if on && to.distance(at) > 1e-6 {
                        let dir = to - at;
                        let angle = dir.y.atan2(dir.x).rem_euclid(2.0 * PI);
                        if !leaving.iter().any(|(a, _): &(f64, FoldKind)| (a - angle).abs() < 1e-6) {
                            leaving.push((angle, c.kind));
                        }
                    }
                }
            }
            leaving.sort_by(|a, b| a.0.total_cmp(&b.0));
            Vertex { at, creases: leaving }
        })
        .filter(|v| v.creases.len() > 1)
        .collect()
}

fn distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn intersection(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Option<DVec2> {
    let (r, s) = (b - a, d - c);
    let denom = crate::fold::cross(r, s);
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = crate::fold::cross(c - a, s) / denom;
    let u = crate::fold::cross(c - a, r) / denom;
    ((-1e-9..=1.0 + 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u)).then(|| a + r * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crease(a: (f64, f64), b: (f64, f64), kind: FoldKind) -> Crease {
        Crease {
            a: DVec2::new(a.0, a.1),
            b: DVec2::new(b.0, b.1),
            kind,
            step: 0,
        }
    }

    #[test]
    fn preliminary_base_center() {
        use FoldKind::*;
        // Valley diagonals and mountain book folds: Kawasaki holds at the center, but 4 M and
        // 4 V break Maekawa; the flattened base reverses one half of a book fold (5 V, 3 M).
        let mut creases = vec![
            crease((-1.0, -1.0), (1.0, 1.0), Valley),
            crease((-1.0, 1.0), (1.0, -1.0), Valley),
            crease((-1.0, 0.0), (1.0, 0.0), Mountain),
            crease((0.0, -1.0), (0.0, 1.0), Mountain),
        ];
        let vertices = inner_vertices(&creases);
        assert_eq!(vertices.len(), 1);
        assert!(vertices[0].kawasaki());
        assert!(!vertices[0].maekawa());
        creases[3] = crease((0.0, 0.0), (0.0, 1.0), Mountain);
        creases.push(crease((0.0, 0.0), (0.0, -1.0), Valley));
        assert!(inner_vertices(&creases)[0].flat_foldable());
    }

    #[test]
    fn three_valleys_one_mountain() {
        use FoldKind::*;
        // A single vertex with four creases at right angles: 3 V + 1 M folds flat.
        let creases = [
            crease((0.0, 0.0), (1.0, 0.0), Valley),
            crease((0.0, 0.0), (0.0, 1.0), Valley),
            crease((0.0, 0.0), (-1.0, 0.0), Valley),
            crease((0.0, 0.0), (0.0, -1.0), Mountain),
        ];
        let vertices = inner_vertices(&creases);
        assert_eq!(vertices.len(), 1);
        assert!(vertices[0].flat_foldable());
    }
}
