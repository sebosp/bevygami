//! The motion of an inside reverse fold, as stiff paper can do it. The two halves of the flap are
//! joined along its spine: turned each around the fold line, one over and one under, the spine
//! would tear in the middle of the motion. Folded paper moves like a vertex of four creases (the
//! spine of the flap, the fold line on its two halves, the spine of the part that stays): the
//! layers that stay open around their own spine, the spine of the flap swings between them, in the
//! middle plane, from where it was to where it lands, and the layers close again. Where it starts
//! on the spine of the layers that stay, they open flat first: the hands open the layers, then
//! push the corner in, as in the diagrams.
//!
//! For each position of the spine of the flap (angle `μ` from where it was), the opening of the
//! layers that stay (`φ`, each half turned by it around their spine) follows from the angle
//! between the fold line and the spine on the flap, which stays the same:
//! `(R_s(φ) l) · m(μ) = l · m(0)`.
use bevy::math::{DMat3, DVec2, DVec3};

use crate::fold::{Paper, cross, shared_edges};

/// What a face of the fold does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The front half of the layers that stay, opens toward the viewer.
    FrontStay,
    BackStay,
    /// The front half of the flap that reverses.
    FrontFlap,
    BackFlap,
    /// Joined to the rest only along the line the layers open around, it does not move.
    Still,
}

/// A reverse fold that moves rigidly, on the folded plane.
#[derive(Debug, Clone, PartialEq)]
pub struct Reverse {
    /// Per face of the plan.
    pub parts: Vec<Part>,
    /// Where the fold line meets the spine of the flap.
    pub vertex: DVec2,
    /// The spine of the layers that stay, the axis they open around, from the vertex.
    stay: DVec2,
    /// Perpendicular to it, toward the layers that stay.
    side: DVec2,
    /// The fold line.
    line: DVec2,
    /// The spine of the flap, from the vertex.
    spine: DVec2,
    /// The angle the spine of the flap turns by, signed, through the inside of the flap.
    sweep: f64,
    /// The opening of the layers at the start and at the end of the swing.
    open_start: f64,
    open_end: f64,
}

/// Where a reverse fold is at a time of its motion: the rotations of the four parts around the
/// vertex, the plane of the paper through it.
pub struct Pose3 {
    pub front_stay: DMat3,
    pub back_stay: DMat3,
    pub front_flap: DMat3,
    pub back_flap: DMat3,
}

impl Pose3 {
    pub fn of(&self, part: Part) -> DMat3 {
        match part {
            Part::FrontStay => self.front_stay,
            Part::BackStay => self.back_stay,
            Part::FrontFlap => self.front_flap,
            Part::BackFlap => self.back_flap,
            Part::Still => DMat3::IDENTITY,
        }
    }
}

const EPS: f64 = 1e-6;
/// The layers may open a little past flat, `cos φ` down to this.
const PAST_FLAT: f64 = -0.5;

fn perp(v: DVec2) -> DVec2 {
    DVec2::new(-v.y, v.x)
}

fn rotate(v: DVec2, a: f64) -> DVec2 {
    DVec2::from_angle(a).rotate(v)
}

fn d3(v: DVec2) -> DVec3 {
    DVec3::new(v.x, v.y, 0.0)
}

/// The orthonormal frame of `u` and of `v` made perpendicular to it.
fn frame(u: DVec3, v: DVec3) -> DMat3 {
    let e1 = u.normalize();
    let e2 = (v - e1 * v.dot(e1)).normalize();
    DMat3::from_cols(e1, e2, e1.cross(e2))
}

/// The direction of the segments, all on one line through `through`, or `None`.
fn common_line(segments: &[(DVec2, DVec2)], through: Option<DVec2>) -> Option<(DVec2, DVec2)> {
    let (p, q) = *segments.first()?;
    let d = (q - p).normalize_or_zero();
    if d == DVec2::ZERO {
        return None;
    }
    let on = |x: DVec2| cross(d, x - p).abs() < EPS;
    let all = segments.iter().all(|(a, b)| on(*a) && on(*b));
    (all && through.is_none_or(on)).then_some((p, d))
}

impl Reverse {
    /// The rigid motion of the reverse fold of the faces that move, the fold line `(a, d)` on the
    /// folded plane, `front` the faces on the front half of the flap. The layers that stay open
    /// around a line through the vertex, the ones that have to: those joined to the halves of the
    /// flap, and on through the paper wherever they are joined away from that line. `None` when
    /// they cannot, the fold then turns the halves of the flap on their own.
    pub fn new(
        paper: &Paper,
        moving: &[bool],
        front: &dyn Fn(usize) -> bool,
        (a, d): (DVec2, DVec2),
    ) -> Option<Reverse> {
        let n = paper.faces.len();
        // Every joint of two faces, on the folded plane.
        let mut joints: Vec<(usize, usize, DVec2, DVec2)> = Vec::new();
        for (i, f) in paper.faces.iter().enumerate() {
            for (j, g) in paper.faces.iter().enumerate().skip(i + 1) {
                for (p, q) in shared_edges(f, g) {
                    joints.push((i, j, f.pose.apply(p), f.pose.apply(q)));
                }
            }
        }
        let flap_part = |i: usize| if front(i) { Part::FrontFlap } else { Part::BackFlap };
        let spine: Vec<(DVec2, DVec2)> = joints
            .iter()
            .filter(|(i, j, _, _)| moving[*i] && moving[*j] && front(*i) != front(*j))
            .map(|(_, _, p, q)| (*p, *q))
            .collect();
        // The vertex: the spine of the flap across the fold line.
        let (sp, sd) = common_line(&spine, None)?;
        let den = cross(sd, d);
        if den.abs() < EPS {
            return None;
        }
        let vertex = sp + sd * (cross(a - sp, d) / den);
        let away = |segments: &[(DVec2, DVec2)], dir: DVec2| {
            let mid = segments.iter().map(|(p, q)| (*p + *q) / 2.0).sum::<DVec2>() / segments.len() as f64;
            if (mid - vertex).dot(dir) >= 0.0 { dir } else { -dir }
        };
        let m0 = away(&spine, sd);
        // The lines the layers that stay may open around: on from the spine of the flap, or along
        // a joint of them through the vertex.
        let mut axes = vec![-m0];
        for (i, j, p, q) in &joints {
            if moving[*i] || moving[*j] {
                continue;
            }
            if let Some((_, dir)) = common_line(&[(*p, *q)], Some(vertex)) {
                let dir = away(&[(*p, *q)], dir);
                if axes.iter().all(|x| x.distance(dir) > EPS) {
                    axes.push(dir);
                }
            }
        }
        let centroid = |faces: &[usize]| {
            (!faces.is_empty())
                .then(|| faces.iter().map(|i| paper.faces[*i].centroid_image()).sum::<DVec2>() / faces.len() as f64)
        };
        let flap_faces: Vec<usize> = (0..n).filter(|i| moving[*i] && front(*i)).collect();
        // The spine swings through the flap, past the fold line on its side.
        let flap = cross(m0, centroid(&flap_faces)? - vertex).signum();
        let l = if cross(m0, d) * flap > 0.0 { d } else { -d };
        let sweep = 2.0 * m0.angle_to(l);
        axes.into_iter().find_map(|s| {
            let parts = Self::open_around(n, &joints, moving, &flap_part, vertex, s)?;
            // The layers lift on the side of the ones joined to the front of the flap.
            let seeds: Vec<usize> = joints
                .iter()
                .filter_map(|(i, j, _, _)| match (parts[*i], parts[*j]) {
                    (Part::FrontFlap, Part::FrontStay) => Some(*j),
                    (Part::FrontStay, Part::FrontFlap) => Some(*i),
                    _ => None,
                })
                .collect();
            let toward = cross(s, centroid(&seeds)? - vertex);
            if toward.abs() < EPS {
                return None;
            }
            Self::swing(parts, vertex, s, perp(s) * toward.signum(), l, m0, sweep)
        })
    }

    /// The faces that open with each half of the flap, around the line `s` from the vertex: the
    /// ones joined to them away from that line; the others do not move.
    fn open_around(
        n: usize,
        joints: &[(usize, usize, DVec2, DVec2)],
        moving: &[bool],
        flap_part: &dyn Fn(usize) -> Part,
        vertex: DVec2,
        s: DVec2,
    ) -> Option<Vec<Part>> {
        let mut parts: Vec<Option<Part>> = (0..n).map(|i| moving[i].then(|| flap_part(i))).collect();
        let stay_of = |part: Part| match part {
            Part::FrontFlap | Part::FrontStay => Part::FrontStay,
            _ => Part::BackStay,
        };
        let on_axis = |p: DVec2, q: DVec2| cross(s, p - vertex).abs() < EPS && cross(s, q - vertex).abs() < EPS;
        let mut todo: Vec<usize> = (0..n).filter(|i| moving[*i]).collect();
        while let Some(i) = todo.pop() {
            let here = stay_of(parts[i]?);
            for (f, g, p, q) in joints {
                let other = if *f == i {
                    *g
                } else if *g == i {
                    *f
                } else {
                    continue;
                };
                if moving[other] {
                    // A half of the flap joined to the other half of what stays.
                    if stay_of(parts[other]?) != here && !moving[i] {
                        return None;
                    }
                    continue;
                }
                if !moving[i] && on_axis(*p, *q) {
                    continue;
                }
                match parts[other] {
                    None => {
                        parts[other] = Some(here);
                        todo.push(other);
                    }
                    Some(there) if there != here => return None,
                    Some(_) => {}
                }
            }
        }
        Some(parts.into_iter().map(|p| p.unwrap_or(Part::Still)).collect())
    }

    /// The motion around the vertex, `None` when the layers would have to open too far.
    fn swing(
        parts: Vec<Part>,
        vertex: DVec2,
        s: DVec2,
        side: DVec2,
        l: DVec2,
        m0: DVec2,
        sweep: f64,
    ) -> Option<Reverse> {
        let mut reverse = Reverse {
            parts,
            vertex,
            stay: s,
            side,
            line: l,
            spine: m0,
            sweep,
            open_start: 0.0,
            open_end: 0.0,
        };
        // The opening along the swing, a little past flat at most.
        let steps = 64;
        for k in 1..steps {
            let c = reverse.cos_open(sweep * k as f64 / steps as f64)?;
            if !(PAST_FLAT..=1.0 + EPS).contains(&c) {
                return None;
            }
        }
        // At the ends, where the spine of the flap lies on the spine of the layers, any opening
        // fits: the layers open before the swing, or close after it.
        let small = 1e-4 * sweep.signum();
        let open = |mu: f64, near: f64| reverse.cos_open(mu).or_else(|| reverse.cos_open(near));
        let (start, end) = (open(0.0, small)?, open(sweep, sweep - small)?);
        reverse.open_start = start.clamp(PAST_FLAT, 1.0).acos();
        reverse.open_end = end.clamp(PAST_FLAT, 1.0).acos();
        Some(reverse)
    }

    /// `cos φ` with the spine of the flap turned by `mu`.
    fn cos_open(&self, mu: f64) -> Option<f64> {
        let m = rotate(self.spine, mu);
        let (a, b) = (self.line.dot(self.stay), self.line.dot(self.side));
        let den = b * self.side.dot(m);
        if den.abs() < EPS {
            return None;
        }
        Some((self.line.dot(self.spine) - a * self.stay.dot(m)) / den)
    }

    /// The opening and the angle of the spine at `t` from 0 to 1: the layers open, the spine
    /// swings, they close, each for its share of the way.
    fn state(&self, t: f64) -> (f64, f64) {
        let (open, swing, close) = (self.open_start, self.sweep.abs(), self.open_end);
        let total = open + swing + close;
        let x = t.clamp(0.0, 1.0) * total;
        if x <= open {
            (x, 0.0)
        } else if x <= open + swing {
            let mu = self.sweep.signum() * (x - open);
            let phi = self.cos_open(mu).map_or(open, |c| c.clamp(PAST_FLAT, 1.0).acos());
            (phi, mu)
        } else {
            (close - (x - open - swing), self.sweep)
        }
    }

    /// The rotations of the parts around the vertex at `t`.
    pub fn at(&self, t: f64) -> Pose3 {
        let (phi, mu) = self.state(t);
        let (s, n) = (d3(self.stay), d3(self.side));
        // Turned around this axis, the side of the layers comes up.
        let axis = if s.cross(n).z > 0.0 { s } else { -s };
        let front_stay = DMat3::from_axis_angle(axis, phi);
        let back_stay = DMat3::from_axis_angle(axis, -phi);
        let l = d3(self.line);
        let m = d3(rotate(self.spine, mu));
        let front_flap = frame(front_stay * l, m) * frame(l, d3(self.spine)).transpose();
        // The back half is the front one seen in the mirror of the middle plane.
        let mirror = DMat3::from_diagonal(DVec3::new(1.0, 1.0, -1.0));
        let back_flap = mirror * front_flap * mirror;
        // The back half stays on the table, the front one lifts: all turned back with it.
        let down = back_stay.transpose();
        Pose3 {
            front_stay: down * front_stay,
            back_stay: DMat3::IDENTITY,
            front_flap: down * front_flap,
            back_flap: down * back_flap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold::plan_all;
    use crate::lessons;

    /// Every edge between two faces stays joined along the whole motion, for the reverse folds
    /// that move rigidly.
    #[test]
    fn reverse_folds_keep_the_paper_joined() {
        let mut rigid = 0;
        for lesson in lessons::all() {
            let folds = lesson.folds();
            let (plans, _) = plan_all(lesson.start(), &folds).expect("folds");
            for (k, plan) in plans.iter().enumerate() {
                let Some(reverse) = &plan.reverse else { continue };
                rigid += 1;
                let faces = &plan.before.faces;
                for t in [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
                    let pose = reverse.at(t);
                    let place = |i: usize, p: DVec2| {
                        let q = faces[i].pose.apply(p) - reverse.vertex;
                        pose.of(reverse.parts[i]) * d3(q)
                    };
                    for i in 0..faces.len() {
                        for j in i + 1..faces.len() {
                            for (p, q) in shared_edges(&faces[i], &faces[j]) {
                                for x in [p, q] {
                                    let gap = place(i, x).distance(place(j, x));
                                    assert!(gap < 1e-6, "{} step {}: torn by {gap} at t {t}", lesson.title, k + 1);
                                }
                            }
                        }
                    }
                }
                // It ends where the fold puts the faces.
                let end = reverse.at(1.0);
                for (i, face) in faces.iter().enumerate() {
                    let landed = &plan.landed.faces[i];
                    for p in &face.poly {
                        let q = end.of(reverse.parts[i]) * d3(face.pose.apply(*p) - reverse.vertex);
                        let want = landed.pose.apply(*p) - reverse.vertex;
                        assert!((q.truncate() - want).length() < 1e-6, "{} step {}", lesson.title, k + 1);
                        assert!(q.z.abs() < 1e-6);
                    }
                }
            }
        }
        assert!(rigid >= 4, "only {rigid} reverse folds move rigidly");
    }
}
