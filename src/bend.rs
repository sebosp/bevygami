//! Paper that bends where stiff paper would tear. Each face moves rigidly; where two faces that
//! the paper joins come apart, the edge goes to the middle of where the two put it, and the
//! difference fades into each face over [`REACH`]: the paper bends near the edge, as it does in
//! the hands. A point of the paper is the same point on every face that has it on its edge (the
//! faces cut the sheet), so the edges stay joined whatever the faces do.
//!
//! The faces are meshed finely for it, with the same points on both sides of every edge.
use std::collections::HashMap;

use bevy::math::{DMat3, DVec2, DVec3};

use crate::fold::{Face, Paper, cross};

/// The spacing of the points of the mesh, in paper units (the sheet is 2 wide).
const STEP: f64 = 0.05;
/// How far into a face the bend goes.
pub const REACH: f64 = 0.35;

/// A face meshed for bending, in paper coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Samples {
    /// The points of its edges, in order around: the corners of the faces next to it among them.
    pub boundary: Vec<DVec2>,
    /// The points of the mesh, the boundary last.
    pub points: Vec<DVec2>,
    pub triangles: Vec<[u32; 3]>,
}

/// The faces of a paper meshed for bending.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub faces: Vec<Samples>,
}

/// Where the paper bends at a time: per face, the correction at each point of its boundary.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bent {
    pub boundary: Vec<Vec<DVec3>>,
}

fn key(p: DVec2) -> (i64, i64) {
    ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64)
}

impl Sheet {
    pub fn new(paper: &Paper) -> Sheet {
        let corners: Vec<DVec2> = paper.faces.iter().flat_map(|f| f.poly.iter().copied()).collect();
        Sheet {
            faces: paper.faces.iter().map(|f| samples(f, &corners)).collect(),
        }
    }

    /// The corrections when each face puts the points of the paper at `place(face, point)`.
    pub fn bend(&self, place: &dyn Fn(usize, DVec2) -> DVec3) -> Bent {
        let mut seams: HashMap<(i64, i64), (DVec3, f64)> = HashMap::new();
        let placed: Vec<Vec<DVec3>> = self
            .faces
            .iter()
            .enumerate()
            .map(|(i, f)| f.boundary.iter().map(|u| place(i, *u)).collect())
            .collect();
        for (f, at) in self.faces.iter().zip(&placed) {
            for (u, x) in f.boundary.iter().zip(at) {
                let seam = seams.entry(key(*u)).or_insert((DVec3::ZERO, 0.0));
                seam.0 += *x;
                seam.1 += 1.0;
            }
        }
        Bent {
            boundary: self
                .faces
                .iter()
                .zip(&placed)
                .map(|(f, at)| {
                    f.boundary
                        .iter()
                        .zip(at)
                        .map(|(u, x)| {
                            let (sum, count) = seams[&key(*u)];
                            sum / count - *x
                        })
                        .collect()
                })
                .collect(),
        }
    }
}

impl Bent {
    /// The face does not bend.
    pub fn flat(&self, face: usize) -> bool {
        self.boundary
            .get(face)
            .is_none_or(|c| c.iter().all(|c| c.length_squared() < 1e-14))
    }

    /// The correction at the point `u` of the face, from those at its boundary: weighted by the
    /// inverse square distance, fading to nothing [`REACH`] away from the edges.
    pub fn at(&self, sheet: &Sheet, face: usize, u: DVec2) -> DVec3 {
        if self.flat(face) {
            return DVec3::ZERO;
        }
        let boundary = &sheet.faces[face].boundary;
        let corrections = &self.boundary[face];
        let mut sum = DVec3::ZERO;
        let mut weights = 0.0;
        let mut nearest = f64::MAX;
        for (b, c) in boundary.iter().zip(corrections) {
            let d2 = b.distance_squared(u);
            if d2 < 1e-18 {
                return *c;
            }
            nearest = nearest.min(d2);
            let w = 1.0 / (d2 * d2);
            sum += *c * w;
            weights += w;
        }
        let fade = (1.0 - nearest.sqrt() / REACH).max(0.0);
        sum / weights * (fade * fade * (3.0 - 2.0 * fade))
    }
}

/// The face meshed in rings around its centroid, its edges cut at the corners of the other faces
/// on them and every [`STEP`] between.
fn samples(face: &Face, corners: &[DVec2]) -> Samples {
    let poly = &face.poly;
    let n = poly.len();
    let mut boundary = Vec::new();
    for e in 0..n {
        let (a, b) = (poly[e], poly[(e + 1) % n]);
        let ab = b - a;
        let len = ab.length();
        let mut cuts = vec![0.0, 1.0];
        for c in corners {
            let t = (*c - a).dot(ab) / (len * len);
            if t > 1e-9 && t < 1.0 - 1e-9 && cross(ab, *c - a).abs() / len < 1e-9 {
                cuts.push(t);
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        for w in cuts.windows(2) {
            let m = ((len * (w[1] - w[0])) / STEP).ceil().max(1.0) as usize;
            for k in 0..m {
                boundary.push(a + ab * (w[0] + (w[1] - w[0]) * k as f64 / m as f64));
            }
        }
    }
    let center = poly.iter().sum::<DVec2>() / n as f64;
    let far = boundary.iter().map(|b| b.distance(center)).fold(0.0, f64::max);
    let rings = (far / STEP).ceil().max(1.0) as usize;
    let m = boundary.len();
    let mut points = vec![center];
    for k in 1..rings {
        let s = k as f64 / rings as f64;
        points.extend(boundary.iter().map(|b| center + (*b - center) * s));
    }
    points.extend(boundary.iter().copied());
    let at = |k: usize, j: usize| (1 + (k - 1) * m + j % m) as u32;
    let mut triangles = Vec::new();
    for j in 0..m {
        triangles.push([0, at(1, j), at(1, j + 1)]);
        for k in 1..rings {
            triangles.push([at(k, j), at(k + 1, j), at(k + 1, j + 1)]);
            triangles.push([at(k, j), at(k + 1, j + 1), at(k, j + 1)]);
        }
    }
    Samples {
        boundary,
        points,
        triangles,
    }
}

/// Where a face is: the point `u` of the paper at `m (u, 0) + t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub m: DMat3,
    pub t: DVec3,
}

impl Place {
    pub fn apply(&self, u: DVec2) -> DVec3 {
        self.m * DVec3::new(u.x, u.y, 0.0) + self.t
    }
}

/// The orthogonal factor of `m` (polar decomposition), by Newton's iteration.
fn orthogonal(mut m: DMat3) -> DMat3 {
    for _ in 0..12 {
        let inv = m.inverse().transpose();
        if !inv.is_finite() {
            break;
        }
        m = (m + inv) * 0.5;
    }
    m
}

impl Sheet {
    /// The faces that move brought together where the paper joins them: each fitted, as a rigid
    /// piece, onto the middle of where it and its neighbors put its edges, again and again. The
    /// faces that do not move hold the rest. What is left apart, the paper bends.
    pub fn relax(&self, moving: &[bool], places: &mut [Place], rounds: usize) {
        for _ in 0..rounds {
            let mut seams: HashMap<(i64, i64), (DVec3, f64)> = HashMap::new();
            for (f, place) in self.faces.iter().zip(places.iter()) {
                for u in &f.boundary {
                    let seam = seams.entry(key(*u)).or_insert((DVec3::ZERO, 0.0));
                    seam.0 += place.apply(*u);
                    seam.1 += 1.0;
                }
            }
            for (i, f) in self.faces.iter().enumerate() {
                if !moving[i] {
                    continue;
                }
                let targets: Vec<DVec3> = f
                    .boundary
                    .iter()
                    .map(|u| {
                        let (sum, count) = seams[&key(*u)];
                        sum / count
                    })
                    .collect();
                let k = targets.len() as f64;
                let (uc, yc) = (f.boundary.iter().sum::<DVec2>() / k, targets.iter().sum::<DVec3>() / k);
                // The best turn: the cross covariance, with the normal kept as it was.
                let mut m = DMat3::ZERO;
                for (u, y) in f.boundary.iter().zip(&targets) {
                    let d = *u - uc;
                    let x = DVec3::new(d.x, d.y, 0.0);
                    m += DMat3::from_cols(*y - yc, DVec3::ZERO, DVec3::ZERO)
                        * DMat3::from_cols(x, DVec3::ZERO, DVec3::ZERO).transpose();
                }
                let scale = f
                    .boundary
                    .iter()
                    .map(|u| u.distance_squared(uc))
                    .sum::<f64>()
                    .max(1e-12);
                m += DMat3::from_cols(DVec3::ZERO, DVec3::ZERO, places[i].m.z_axis * scale);
                let r = orthogonal(m);
                if r.determinant() > 0.5 {
                    places[i] = Place {
                        m: r,
                        t: yc - r * DVec3::new(uc.x, uc.y, 0.0),
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold::plan_all;
    use crate::lessons;

    /// The reverse folds that bend keep every edge joined, and end where the fold puts them.
    #[test]
    fn bending_keeps_the_paper_joined() {
        let mut bending = 0;
        for lesson in lessons::all() {
            let (plans, _) = plan_all(lesson.start(), &lesson.folds()).expect("folds");
            for (k, plan) in plans.iter().enumerate() {
                let Some(reverse) = plan.reverse.as_ref().filter(|r| r.bending) else {
                    continue;
                };
                bending += 1;
                let faces = &plan.before.faces;
                let sheet = Sheet::new(&plan.before);
                for t in [0.0, 0.2, 0.5, 0.8, 1.0] {
                    let pose = reverse.at(t);
                    let rigid = |i: usize, u: DVec2| {
                        let q = faces[i].pose.apply(u) - reverse.vertex;
                        pose.of(reverse.parts[i]) * DVec3::new(q.x, q.y, 0.0)
                    };
                    let bent = sheet.bend(&rigid);
                    let place = |i: usize, u: DVec2| rigid(i, u) + bent.at(&sheet, i, u);
                    for i in 0..faces.len() {
                        for j in i + 1..faces.len() {
                            for (p, q) in crate::fold::shared_edges(&faces[i], &faces[j]) {
                                for x in [p, q] {
                                    let gap = place(i, x).distance(place(j, x));
                                    assert!(gap < 1e-6, "{} step {}: torn by {gap} at t {t}", lesson.title, k + 1);
                                }
                            }
                        }
                    }
                    if t == 1.0 {
                        assert!(
                            (0..faces.len()).all(|i| bent.flat(i)),
                            "{} step {}",
                            lesson.title,
                            k + 1
                        );
                    }
                }
            }
        }
        assert_eq!(bending, 2, "the tail and the head of the crane bend");
    }

    /// The mesh of a face covers it: the triangles add up to its area.
    #[test]
    fn the_mesh_covers_the_face() {
        let paper = lessons::all()[0].start();
        let sheet = Sheet::new(&paper);
        let face = &sheet.faces[0];
        let area: f64 = face
            .triangles
            .iter()
            .map(|[a, b, c]| {
                let (a, b, c) = (
                    face.points[*a as usize],
                    face.points[*b as usize],
                    face.points[*c as usize],
                );
                cross(b - a, c - a) / 2.0
            })
            .sum();
        assert!((area - paper.faces[0].area()).abs() < 1e-9, "{area}");
    }
}
