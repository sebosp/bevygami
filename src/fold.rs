//! The folding engine, without Bevy: a sheet of paper as flat faces stacked in layers, and the
//! simple folds of the diagrams (valley, mountain, fold and unfold) applied to it.
//!
//! Every face keeps its polygon in the coordinates of the unfolded sheet (the "paper"
//! coordinates, the square `[-1, 1]²`) and a [`Pose`], the isometry that places it on the folded
//! model (the "folded" plane, the one the viewer looks at). A fold reflects the moving faces across
//! the fold line: in 3D that reflection is a half turn around the line, which is how the faces are
//! animated. The layers order the faces from the table (0) up to the viewer.
//!
//! The fold lines are built like Huzita–Hatori axioms on the points of the paper: O1 through two
//! points, O2 one point onto another, O3 one edge onto another. The points are named in paper
//! coordinates, so a lesson says "the bottom corner onto the top corner" whatever the folds
//! before it did; the engine finds where the points are now.
//!
//! The crease records follow ORIPA and Oriedita: a crease is mountain or valley as seen from the
//! front (colored) side of the sheet.
use bevy::math::{DMat2, DVec2};
use serde::{Deserialize, Serialize};

pub const EPS: f64 = 1e-7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FoldKind {
    /// The paper folds toward the viewer, a dashed line in the diagrams.
    Valley,
    /// The paper folds away from the viewer, a dash-dot-dot line in the diagrams.
    Mountain,
}

impl FoldKind {
    pub fn opposite(self) -> Self {
        match self {
            FoldKind::Valley => FoldKind::Mountain,
            FoldKind::Mountain => FoldKind::Valley,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FoldKind::Valley => "valley",
            FoldKind::Mountain => "mountain",
        }
    }
}

/// An isometry of the plane, the reflections included: `p ↦ m p + t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub m: DMat2,
    pub t: DVec2,
}

impl Pose {
    pub const IDENTITY: Pose = Pose {
        m: DMat2::IDENTITY,
        t: DVec2::ZERO,
    };

    pub fn rotation(angle: f64) -> Pose {
        Pose {
            m: DMat2::from_angle(angle),
            t: DVec2::ZERO,
        }
    }

    /// The reflection across the line through `a` with the unit direction `d`.
    pub fn reflection(a: DVec2, d: DVec2) -> Pose {
        let m = DMat2::from_cols(
            DVec2::new(2.0 * d.x * d.x - 1.0, 2.0 * d.x * d.y),
            DVec2::new(2.0 * d.x * d.y, 2.0 * d.y * d.y - 1.0),
        );
        Pose { m, t: a - m * a }
    }

    pub fn apply(&self, p: DVec2) -> DVec2 {
        self.m * p + self.t
    }

    /// `other` after `self`.
    pub fn then(&self, other: &Pose) -> Pose {
        Pose {
            m: other.m * self.m,
            t: other.m * self.t + other.t,
        }
    }

    /// The back of the paper faces the viewer.
    pub fn flipped(&self) -> bool {
        self.m.determinant() < 0.0
    }

    /// The angle of the rotation part: `m` is `R(angle)` or `R(angle) · diag(1, -1)`.
    pub fn angle(&self) -> f64 {
        self.m.x_axis.y.atan2(self.m.x_axis.x)
    }
}

/// A flat convex piece of the sheet between creases.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Counterclockwise, in paper coordinates.
    pub poly: Vec<DVec2>,
    pub pose: Pose,
    pub layer: i32,
}

impl Face {
    pub fn image(&self) -> Vec<DVec2> {
        self.poly.iter().map(|p| self.pose.apply(*p)).collect()
    }

    pub fn area(&self) -> f64 {
        area(&self.poly)
    }

    /// The point is inside or on the border, in paper coordinates.
    pub fn contains(&self, p: DVec2) -> bool {
        let n = self.poly.len();
        (0..n).all(|i| cross(self.poly[(i + 1) % n] - self.poly[i], p - self.poly[i]) >= -EPS)
    }

    pub fn centroid(&self) -> DVec2 {
        self.poly.iter().copied().sum::<DVec2>() / self.poly.len() as f64
    }

    /// The centroid on the folded plane.
    pub fn centroid_image(&self) -> DVec2 {
        self.pose.apply(self.centroid())
    }
}

/// A crease of the paper, a segment in paper coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crease {
    pub a: DVec2,
    pub b: DVec2,
    /// As seen from the front side.
    pub kind: FoldKind,
    /// The step that made it.
    pub step: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paper {
    pub faces: Vec<Face>,
    pub creases: Vec<Crease>,
}

impl Paper {
    /// The square `[-1, 1]²`, placed by `pose` (a diamond is the square turned by 45°).
    pub fn square(pose: Pose) -> Paper {
        let poly = vec![
            DVec2::new(-1.0, -1.0),
            DVec2::new(1.0, -1.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(-1.0, 1.0),
        ];
        Paper {
            faces: vec![Face { poly, pose, layer: 0 }],
            creases: Vec::new(),
        }
    }

    /// Where a point of the paper is on the folded model.
    pub fn image_of(&self, p: DVec2) -> Option<DVec2> {
        self.face_at(p).map(|f| self.faces[f].pose.apply(p))
    }

    pub fn face_at(&self, p: DVec2) -> Option<usize> {
        self.faces.iter().position(|f| f.contains(p))
    }

    pub fn max_layer(&self) -> i32 {
        self.faces.iter().map(|f| f.layer).max().unwrap_or(0)
    }

    /// The outline of the folded model, its bounding box.
    pub fn bounds(&self) -> (DVec2, DVec2) {
        let mut min = DVec2::splat(f64::MAX);
        let mut max = DVec2::splat(f64::MIN);
        for p in self.faces.iter().flat_map(|f| f.image()) {
            min = min.min(p);
            max = max.max(p);
        }
        (min, max)
    }
}

/// How a fold line is found, from points of the paper (paper coordinates) or of the folded model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Line {
    /// O1: the line through two points.
    Through(DVec2, DVec2),
    /// O2: the line that brings the first point onto the second, their perpendicular bisector.
    Bring(DVec2, DVec2),
    /// O3: the line that brings the edge `(a, b)` onto the edge `(c, d)`: the bisector of their
    /// angle that sends `b - a` along `d - c`.
    EdgeOnto(DVec2, DVec2, DVec2, DVec2),
    /// The line through two points of the folded model, where the paper is now: for the steps on
    /// many layers, where a point of the paper is hard to name.
    Folded(DVec2, DVec2),
}

impl Line {
    /// The same line on the sheet mirrored by `x ↦ -x`, see [`Fold::mirrored`].
    fn mirrored(self) -> Line {
        let m = |p: DVec2| DVec2::new(-p.x, p.y);
        match self {
            Line::Through(p, q) => Line::Through(m(p), m(q)),
            Line::Bring(p, q) => Line::Bring(m(p), m(q)),
            Line::EdgeOnto(a, b, c, d) => Line::EdgeOnto(m(a), m(b), m(c), m(d)),
            Line::Folded(p, q) => Line::Folded(p, q),
        }
    }
}

/// The point that moves, which picks the side of the line that folds and the flap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grip {
    /// A point of the paper.
    Paper(DVec2),
    /// The top layer at a point of the folded model.
    Top(DVec2),
}

impl From<DVec2> for Grip {
    fn from(p: DVec2) -> Grip {
        Grip::Paper(p)
    }
}

/// Which layers fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// Every layer on the moving side of the line.
    #[default]
    All,
    /// Only the flap that holds the moving point: the faces on the moving side joined to it by
    /// the paper itself.
    Flap,
}

/// How the moving layers go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    /// All together, valley or mountain.
    #[default]
    Simple,
    /// A flap of two halves pushed inside itself, between its layers: the front half goes back,
    /// the back half comes forward, the spine between them reverses.
    InsideReverse,
    /// A flap of two halves wrapped around itself: the front half comes forward, the back half
    /// goes back.
    OutsideReverse,
    /// The whole model turned over, left to right.
    TurnOver,
    /// Several creases fold at once, see [`Collapse`].
    Collapse,
}

/// A step where several creases fold at once (a petal fold, a squash, the collapse of a crease
/// pattern), as the CP editors fold a crease pattern: the creases listed fold, the ones listed
/// flat open, every other crease keeps its state; the faces are placed by walking the paper from
/// the face at the grip, which stays, reflecting across each folded crease crossed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collapse {
    /// The creases that fold, in paper coordinates, mountain or valley seen from the front side
    /// as in a crease pattern; for a sheet turned over (`mirror`), from the back, the side up.
    pub fold: &'static [(DVec2, DVec2, FoldKind)],
    /// The creases that open flat.
    pub flat: &'static [(DVec2, DVec2)],
    /// The sheet mirrored by `x ↦ -x`, see [`Fold::mirrored`]: turned over, its creases are
    /// mountain where they were valley.
    pub mirror: bool,
}

impl Collapse {
    fn point(&self, p: DVec2) -> DVec2 {
        if self.mirror { DVec2::new(-p.x, p.y) } else { p }
    }
}

/// A step of a diagram.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fold {
    pub line: Line,
    pub moving: Grip,
    pub kind: FoldKind,
    pub scope: Scope,
    pub style: Style,
    /// Fold and unfold: only a crease is left, a precrease.
    pub unfold: bool,
    pub collapse: Option<Collapse>,
}

impl Fold {
    pub fn valley(line: Line, moving: impl Into<Grip>) -> Fold {
        Fold {
            line,
            moving: moving.into(),
            kind: FoldKind::Valley,
            scope: Scope::All,
            style: Style::Simple,
            unfold: false,
            collapse: None,
        }
    }

    pub fn mountain(line: Line, moving: impl Into<Grip>) -> Fold {
        Fold {
            kind: FoldKind::Mountain,
            ..Fold::valley(line, moving)
        }
    }

    /// One point onto another, valley, the first point moves.
    pub fn bring(from: DVec2, to: DVec2) -> Fold {
        Fold::valley(Line::Bring(from, to), from)
    }

    /// An inside reverse fold of the flap at `moving`.
    pub fn inside_reverse(line: Line, moving: impl Into<Grip>) -> Fold {
        Fold {
            style: Style::InsideReverse,
            scope: Scope::Flap,
            ..Fold::valley(line, moving)
        }
    }

    /// An outside reverse fold of the flap at `moving`.
    pub fn outside_reverse(line: Line, moving: impl Into<Grip>) -> Fold {
        Fold {
            style: Style::OutsideReverse,
            scope: Scope::Flap,
            ..Fold::valley(line, moving)
        }
    }

    /// Turns the model over, left to right.
    pub fn turn_over() -> Fold {
        Fold {
            style: Style::TurnOver,
            ..Fold::valley(Line::Folded(DVec2::ZERO, DVec2::Y), Grip::Top(DVec2::ZERO))
        }
    }

    pub fn flap(self) -> Fold {
        Fold {
            scope: Scope::Flap,
            ..self
        }
    }

    pub fn and_unfold(self) -> Fold {
        Fold { unfold: true, ..self }
    }

    /// The same fold on the sheet mirrored by `x ↦ -x`: a lesson that starts with the sheet turned
    /// over (white side up) names the same corners where they are seen.
    pub fn mirrored(self) -> Fold {
        let moving = match self.moving {
            Grip::Paper(p) => Grip::Paper(DVec2::new(-p.x, p.y)),
            top => top,
        };
        let collapse = self.collapse.map(|c| Collapse { mirror: !c.mirror, ..c });
        Fold {
            line: self.line.mirrored(),
            moving,
            collapse,
            ..self
        }
    }

    /// A collapse, the face at `stays` does not move.
    pub fn collapse(collapse: Collapse, stays: impl Into<Grip>) -> Fold {
        Fold {
            style: Style::Collapse,
            collapse: Some(collapse),
            ..Fold::valley(Line::Folded(DVec2::ZERO, DVec2::X), stays)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FoldError {
    /// A point of the line is not on the paper.
    PointOffPaper(DVec2),
    DegenerateLine,
    /// The moving point is on the fold line.
    MovingOnLine,
    NothingMoves,
    /// The creases of a collapse do not fold flat: two ways around the paper put a face in two
    /// places.
    Inconsistent(DVec2),
}

/// A segment of the fold line on a face that stays, in paper coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chord {
    /// The face of [`Planned::before`].
    pub face: usize,
    pub a: DVec2,
    pub b: DVec2,
    /// As seen from the front side of the paper.
    pub kind: FoldKind,
}

/// A fold worked out: the paper cut along the line, the faces that turn and where they land.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub fold: Fold,
    /// The paper split along the fold line, nothing moved yet.
    pub before: Paper,
    /// Per face of `before`.
    pub moving: Vec<bool>,
    /// Per face of `before`: how a moving face turns, as seen by the viewer.
    pub turns: Vec<FoldKind>,
    /// The point of the paper that moves, for the arrow.
    pub grip: DVec2,
    /// The fold line on the folded plane: a point and the unit direction.
    pub axis: (DVec2, DVec2),
    /// The unit normal of the line toward the moving side.
    pub side: DVec2,
    /// The faces of `before` once folded, in the same order (unfolded again for a precrease).
    pub after: Paper,
    /// The faces of `before` once folded, the precreases included, for the landing outline.
    pub landed: Paper,
    pub chords: Vec<Chord>,
    /// How a reverse fold moves without tearing the spine of the flap, when it can.
    pub reverse: Option<crate::motion::Reverse>,
}

pub fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

pub fn area(poly: &[DVec2]) -> f64 {
    let n = poly.len();
    (0..n).map(|i| cross(poly[i], poly[(i + 1) % n])).sum::<f64>() / 2.0
}

/// The fold line on the folded plane, a point and a unit direction.
pub fn locate(paper: &Paper, line: Line) -> Result<(DVec2, DVec2), FoldError> {
    let at = |p: DVec2| paper.image_of(p).ok_or(FoldError::PointOffPaper(p));
    let (a, d) = match line {
        Line::Through(p, q) => {
            let (p, q) = (at(p)?, at(q)?);
            (p, q - p)
        }
        Line::Bring(p, q) => {
            let (p, q) = (at(p)?, at(q)?);
            let v = q - p;
            ((p + q) / 2.0, DVec2::new(-v.y, v.x))
        }
        Line::EdgeOnto(a, b, c, d) => {
            let (a, b, c, d) = (at(a)?, at(b)?, at(c)?, at(d)?);
            let (u, v) = ((b - a).normalize_or_zero(), (d - c).normalize_or_zero());
            if cross(u, v).abs() < EPS {
                // Parallel edges: the line halfway between them.
                ((a + c) / 2.0, u)
            } else {
                // Where the two edges meet.
                let s = cross(c - a, v) / cross(u, v);
                (a + u * s, u + v)
            }
        }
        Line::Folded(p, q) => (p, q - p),
    };
    if d.length() < EPS {
        return Err(FoldError::DegenerateLine);
    }
    Ok((a, d.normalize()))
}

/// The point of the paper under a point of the folded model, on the top layer there.
pub fn top_at(paper: &Paper, q: DVec2) -> Option<DVec2> {
    paper
        .faces
        .iter()
        .filter(|f| {
            let image = f.image();
            let n = image.len();
            let turn = if f.pose.flipped() { -1.0 } else { 1.0 };
            (0..n).all(|i| cross(image[(i + 1) % n] - image[i], q - image[i]) * turn >= -EPS)
        })
        .max_by_key(|f| f.layer)
        .map(|f| f.pose.m.transpose() * (q - f.pose.t))
}

/// Cuts a face along the line: the part on the left of `d` and the part on the right.
fn split(face: &Face, a: DVec2, d: DVec2) -> (Option<Face>, Option<Face>) {
    let s: Vec<f64> = face.poly.iter().map(|p| cross(d, face.pose.apply(*p) - a)).collect();
    let part = |sign: f64| {
        let n = face.poly.len();
        let mut poly = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            let (si, sj) = (s[i] * sign, s[j] * sign);
            if si >= -EPS {
                poly.push(face.poly[i]);
            }
            if (si > EPS && sj < -EPS) || (si < -EPS && sj > EPS) {
                let t = si / (si - sj);
                poly.push(face.poly[i].lerp(face.poly[j], t));
            }
        }
        poly.dedup_by(|p, q| p.distance(*q) < EPS);
        if poly.len() > 1 && poly[0].distance(poly[poly.len() - 1]) < EPS {
            poly.pop();
        }
        (poly.len() >= 3 && area(&poly) > EPS).then(|| Face { poly, ..face.clone() })
    };
    (part(1.0), part(-1.0))
}

/// The common part of two edges on the same line, in paper coordinates.
fn shared_segment(p: (DVec2, DVec2), q: (DVec2, DVec2)) -> Option<(DVec2, DVec2)> {
    let d = p.1 - p.0;
    let len = d.length();
    if len < EPS {
        return None;
    }
    let u = d / len;
    if cross(u, q.0 - p.0).abs() > 1e-6 || cross(u, q.1 - p.0).abs() > 1e-6 {
        return None;
    }
    let (t0, t1) = ((q.0 - p.0).dot(u), (q.1 - p.0).dot(u));
    let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(len));
    (hi - lo > 1e-6).then(|| (p.0 + u * lo, p.0 + u * hi))
}

/// The edges two faces share, in paper coordinates.
pub fn shared_edges(f: &Face, g: &Face) -> Vec<(DVec2, DVec2)> {
    let edges = |face: &Face| {
        let n = face.poly.len();
        (0..n)
            .map(|i| (face.poly[i], face.poly[(i + 1) % n]))
            .collect::<Vec<_>>()
    };
    let (ef, eg) = (edges(f), edges(g));
    ef.iter()
        .flat_map(|e| eg.iter().filter_map(move |h| shared_segment(*e, *h)))
        .collect()
}

/// Works out a fold of the paper, `step` is recorded on the new creases.
pub fn plan(paper: &Paper, fold: Fold, step: usize) -> Result<Planned, FoldError> {
    if fold.style == Style::TurnOver {
        return Ok(turn_over(paper, fold));
    }
    if let Some(collapse) = fold.collapse {
        return plan_collapse(paper, fold, collapse, step);
    }
    let (a, d) = locate(paper, fold.line)?;
    let side_of = |p: DVec2| cross(d, p - a);
    let grip = match fold.moving {
        Grip::Paper(p) => p,
        Grip::Top(q) => top_at(paper, q).ok_or(FoldError::PointOffPaper(q))?,
    };
    let moving_image = paper.image_of(grip).ok_or(FoldError::PointOffPaper(grip))?;
    let sign = side_of(moving_image);
    if sign.abs() < 1e-6 {
        return Err(FoldError::MovingOnLine);
    }
    let sign = sign.signum();
    let side = DVec2::new(-d.y, d.x) * sign;

    // Every face cut in two, the face it comes from, and on which side it is.
    let mut parts: Vec<(Face, usize, bool)> = Vec::new();
    for (i, face) in paper.faces.iter().enumerate() {
        let (left, right) = split(face, a, d);
        for (part, s) in [(left, 1.0), (right, -1.0)] {
            if let Some(part) = part {
                parts.push((part, i, s == sign));
            }
        }
    }
    let mut moving: Vec<bool> = parts.iter().map(|(_, _, on_side)| *on_side).collect();
    if fold.scope == Scope::Flap {
        // The faces joined to the moving point through the paper, on the moving side; the top
        // one when the point is on several.
        let start = parts
            .iter()
            .enumerate()
            .filter(|(_, (part, _, on_side))| *on_side && part.contains(grip))
            .max_by_key(|(_, (part, _, _))| part.layer)
            .map(|(i, _)| i)
            .ok_or(FoldError::NothingMoves)?;
        let mut flap = vec![false; parts.len()];
        flap[start] = true;
        let mut todo = vec![start];
        while let Some(i) = todo.pop() {
            for j in 0..parts.len() {
                if !flap[j] && moving[j] && !shared_edges(&parts[i].0, &parts[j].0).is_empty() {
                    flap[j] = true;
                    todo.push(j);
                }
            }
        }
        moving = flap;
    }
    if !moving.iter().any(|m| *m) {
        return Err(FoldError::NothingMoves);
    }

    // A face that does not need the cut stays whole.
    let mut faces = Vec::new();
    let mut face_moving = Vec::new();
    for (i, face) in paper.faces.iter().enumerate() {
        let mine: Vec<usize> = (0..parts.len()).filter(|j| parts[*j].1 == i).collect();
        let all_same = mine.iter().all(|j| moving[*j] == moving[mine[0]]);
        if all_same {
            faces.push(face.clone());
            face_moving.push(mine.first().is_some_and(|j| moving[*j]));
        } else {
            for j in mine {
                faces.push(parts[j].0.clone());
                face_moving.push(moving[j]);
            }
        }
    }
    let before = Paper {
        faces,
        creases: paper.creases.clone(),
    };

    let (lo, hi) = (
        before.faces.iter().map(|f| f.layer).min().unwrap_or(0),
        before.faces.iter().map(|f| f.layer).max().unwrap_or(0),
    );
    let moving_layers = || {
        before
            .faces
            .iter()
            .zip(&face_moving)
            .filter(|(_, m)| **m)
            .map(|(f, _)| f.layer)
    };
    let (mlo, mhi) = (moving_layers().min().unwrap_or(0), moving_layers().max().unwrap_or(0));
    // The front half of a flap, for the reverse folds: the upper half of its own layers, other
    // layers of the model can lie between them.
    let mut flap_layers: Vec<i32> = moving_layers().collect();
    flap_layers.sort_unstable();
    flap_layers.dedup();
    let k = flap_layers.len();
    let middle = if k.is_multiple_of(2) && k > 0 {
        flap_layers[k / 2 - 1] + flap_layers[k / 2]
    } else {
        mlo + mhi
    };
    let front = |layer: i32| 2 * layer > middle;
    let turns: Vec<FoldKind> = before
        .faces
        .iter()
        .map(|f| match fold.style {
            Style::InsideReverse if front(f.layer) => FoldKind::Mountain,
            Style::InsideReverse => FoldKind::Valley,
            Style::OutsideReverse if front(f.layer) => FoldKind::Valley,
            Style::OutsideReverse => FoldKind::Mountain,
            _ => fold.kind,
        })
        .collect();

    // The fold line runs where a face that moves meets one that stays.
    let mut chords = Vec::new();
    for (i, f) in before.faces.iter().enumerate() {
        if face_moving[i] {
            continue;
        }
        for (j, g) in before.faces.iter().enumerate() {
            if !face_moving[j] {
                continue;
            }
            for (p, q) in shared_edges(f, g) {
                let on_line = side_of(f.pose.apply(p)).abs() < 1e-6 && side_of(f.pose.apply(q)).abs() < 1e-6;
                if on_line {
                    let kind = if f.pose.flipped() {
                        turns[j].opposite()
                    } else {
                        turns[j]
                    };
                    chords.push(Chord {
                        face: i,
                        a: p,
                        b: q,
                        kind,
                    });
                }
            }
        }
    }

    // The moving faces turn over: valley on top of the stack, mountain under it, their order
    // reversed; an inside reverse fold slides each half just inside the layer it was.
    let reflect = Pose::reflection(a, d);
    let mut landed = before.clone();
    for (i, face) in landed.faces.iter_mut().enumerate() {
        let layer = face.layer;
        face.layer = 4 * layer;
        if face_moving[i] {
            face.pose = face.pose.then(&reflect);
            face.layer = match (fold.style, turns[i]) {
                (Style::InsideReverse, _) => 4 * layer + if front(layer) { -1 } else { 1 },
                (_, FoldKind::Valley) => 4 * (hi + 1 + (mhi - layer)),
                (_, FoldKind::Mountain) => 4 * (lo - 1 - (layer - mlo)),
            };
        }
    }
    compact_layers(&mut landed);
    let mut creases = paper.creases.clone();
    if matches!(fold.style, Style::InsideReverse | Style::OutsideReverse) {
        // The spine of the flap, between its two halves, turns inside out.
        for (i, f) in before.faces.iter().enumerate() {
            for (j, g) in before.faces.iter().enumerate().skip(i + 1) {
                if !(face_moving[i] && face_moving[j]) || front(f.layer) == front(g.layer) {
                    continue;
                }
                for (p, q) in shared_edges(f, g) {
                    creases = reverse_part(creases, p, q, step);
                }
            }
        }
    }
    for chord in &chords {
        // A crease folded again takes the new direction.
        creases.retain(|c| !(same_segment((c.a, c.b), (chord.a, chord.b))));
        creases.push(Crease {
            a: chord.a,
            b: chord.b,
            kind: chord.kind,
            step,
        });
    }
    landed.creases = creases.clone();
    let after = if fold.unfold {
        Paper {
            faces: before.faces.clone(),
            creases,
        }
    } else {
        landed.clone()
    };
    let reverse = match fold.style {
        Style::InsideReverse | Style::OutsideReverse => {
            crate::motion::Reverse::new(&before, &face_moving, &|i| front(before.faces[i].layer), (a, d))
        }
        _ => None,
    };
    Ok(Planned {
        reverse,
        fold,
        before,
        moving: face_moving,
        turns,
        grip,
        axis: (a, d),
        side,
        after,
        landed,
        chords,
    })
}

/// The part `(p, q)` of the creases under it turned the other way, the rest of them kept.
fn reverse_part(creases: Vec<Crease>, p: DVec2, q: DVec2, step: usize) -> Vec<Crease> {
    let mut pieces = Vec::new();
    for c in creases {
        if !on_segment((c.a, c.b), p, q) {
            pieces.push(c);
            continue;
        }
        let (p, q) = if p.distance(c.a) <= q.distance(c.a) {
            (p, q)
        } else {
            (q, p)
        };
        for (a, b, reversed) in [(c.a, p, false), (p, q, true), (q, c.b, false)] {
            if a.distance(b) > 1e-6 {
                pieces.push(Crease {
                    a,
                    b,
                    kind: if reversed { c.kind.opposite() } else { c.kind },
                    step: if reversed { step } else { c.step },
                });
            }
        }
    }
    pieces
}

/// The segment `(p, q)` lies on the crease.
fn on_segment(crease: (DVec2, DVec2), p: DVec2, q: DVec2) -> bool {
    let near = |x: DVec2| {
        let ab = crease.1 - crease.0;
        let t = ((x - crease.0).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        x.distance(crease.0 + ab * t) < 1e-6
    };
    near(p) && near(q)
}

/// The whole model turned over around the vertical line through its middle.
fn turn_over(paper: &Paper, fold: Fold) -> Planned {
    let (min, max) = paper.bounds();
    let a = DVec2::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
    let d = DVec2::Y;
    // The arrow goes from the rightmost corner of the top layer.
    let top = paper.max_layer();
    let grip = paper
        .faces
        .iter()
        .filter(|f| f.layer == top)
        .flat_map(|f| f.poly.iter().map(move |p| (*p, f.pose.apply(*p))))
        .max_by(|x, y| x.1.x.total_cmp(&y.1.x))
        .map(|(p, _)| p)
        .unwrap_or(DVec2::ZERO);
    let reflect = Pose::reflection(a, d);
    let (lo, hi) = (
        paper.faces.iter().map(|f| f.layer).min().unwrap_or(0),
        paper.max_layer(),
    );
    let mut landed = paper.clone();
    for face in &mut landed.faces {
        face.pose = face.pose.then(&reflect);
        face.layer = hi + lo - face.layer;
    }
    let after = if fold.unfold { paper.clone() } else { landed.clone() };
    Planned {
        fold,
        before: paper.clone(),
        moving: vec![true; paper.faces.len()],
        turns: vec![FoldKind::Valley; paper.faces.len()],
        grip,
        axis: (a, d),
        side: DVec2::X,
        after,
        landed,
        chords: Vec::new(),
        reverse: None,
    }
}

impl Pose {
    pub fn inverse(&self) -> Pose {
        let m = self.m.transpose();
        Pose { m, t: -(m * self.t) }
    }

    pub fn close_to(&self, other: &Pose) -> bool {
        (self.m - other.m).to_cols_array().iter().all(|x| x.abs() < 1e-6) && self.t.distance(other.t) < 1e-6
    }
}

/// Cuts a face along a line of the paper (paper coordinates).
fn split_paper(face: &Face, a: DVec2, d: DVec2) -> (Option<Face>, Option<Face>) {
    let flat = Face {
        pose: Pose::IDENTITY,
        ..face.clone()
    };
    let (l, r) = split(&flat, a, d);
    let restore = |f: Option<Face>| f.map(|f| Face { pose: face.pose, ..f });
    (restore(l), restore(r))
}

/// The segment crosses the inside of the face (paper coordinates).
fn crosses(face: &Face, a: DVec2, b: DVec2) -> bool {
    let d = (b - a).normalize_or_zero();
    let n = face.poly.len();
    // The part of the segment inside the convex polygon.
    let (mut lo, mut hi) = (0.0f64, a.distance(b));
    for i in 0..n {
        let (p, q) = (face.poly[i], face.poly[(i + 1) % n]);
        let normal = cross(q - p, d);
        let offset = cross(q - p, a - p);
        if normal.abs() < 1e-12 {
            if offset <= 1e-9 {
                return false;
            }
        } else {
            let t = -offset / normal;
            if normal > 0.0 { lo = lo.max(t) } else { hi = hi.min(t) }
        }
    }
    hi - lo > 1e-6
}

/// Convex images overlapping with some area: no separating axis among their edges.
fn overlap(f: &[DVec2], g: &[DVec2]) -> bool {
    for poly in [f, g] {
        let n = poly.len();
        for i in 0..n {
            let e = poly[(i + 1) % n] - poly[i];
            let axis = DVec2::new(-e.y, e.x).normalize_or_zero();
            let span = |p: &[DVec2]| {
                p.iter().fold((f64::MAX, f64::MIN), |(lo, hi), x| {
                    let v = x.dot(axis);
                    (lo.min(v), hi.max(v))
                })
            };
            let (a, b) = (span(f), span(g));
            if a.1 <= b.0 + 1e-6 || b.1 <= a.0 + 1e-6 {
                return false;
            }
        }
    }
    true
}

/// The stacking order of a folded state, as the CP editors find it from the crease pattern, see
/// [`crate::layers`]: two faces on both sides of a folded crease face each other with their front
/// sides across a valley (back sides across a mountain), the faces that do not move keep their
/// order, so do faces that moved together in one piece (reversed when turned over); `hint` orders
/// the rest. Returns the layers, 0 at the bottom.
fn stack(before: &Paper, after: &[Face], kinds: &dyn Fn(DVec2, DVec2) -> Option<FoldKind>, hint: &[i32]) -> Vec<i32> {
    use crate::layers::{Problem, ccw, clip, through};
    let n = after.len();
    let images: Vec<Vec<DVec2>> = after.iter().map(|f| ccw(&f.image())).collect();
    let overlaps: Vec<Vec<bool>> = (0..n)
        .map(|i| (0..n).map(|j| i != j && overlap(&images[i], &images[j])).collect())
        .collect();
    let before_images: Vec<Vec<DVec2>> = before.faces.iter().map(|f| ccw(&f.image())).collect();
    // The faces that moved together, joined through the paper: one rigid piece each.
    let delta: Vec<Pose> = (0..n)
        .map(|i| before.faces[i].pose.inverse().then(&after[i].pose))
        .collect();
    let mut piece: Vec<usize> = (0..n).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            for j in 0..n {
                if piece[j] < piece[i]
                    && delta[i].close_to(&delta[j])
                    && !shared_edges(&before.faces[i], &before.faces[j]).is_empty()
                {
                    piece[i] = piece[j];
                    changed = true;
                }
            }
        }
    }
    let mut problem = Problem::new(&overlaps);
    // The folded edges and the flat joints: the two faces and the edge on the folded plane.
    let mut edges: Vec<(usize, usize, DVec2, DVec2)> = Vec::new();
    let mut joints: Vec<(usize, usize, DVec2, DVec2)> = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            let (f, g) = (&after[i], &after[j]);
            let flat = f.pose.close_to(&g.pose);
            for (p, q) in shared_edges(f, g) {
                let (a, b) = (f.pose.apply(p), f.pose.apply(q));
                if flat {
                    joints.push((i, j, a, b));
                    continue;
                }
                edges.push((i, j, a, b));
                if !overlaps[i][j] {
                    continue;
                }
                if let Some(kind) = kinds(p, q) {
                    let front_up = !f.pose.flipped();
                    if (kind == FoldKind::Valley) == front_up {
                        problem.require_over(j, i);
                    } else {
                        problem.require_over(i, j);
                    }
                }
            }
        }
    }
    let identity = Pose::IDENTITY;
    for i in 0..n {
        for j in 0..n {
            if !overlaps[i][j] || before.faces[i].layer <= before.faces[j].layer {
                continue;
            }
            // Their old order only means something where they already overlapped.
            if !overlap(&before_images[i], &before_images[j]) {
                continue;
            }
            let still = delta[i].close_to(&identity) && delta[j].close_to(&identity);
            if still || piece[i] == piece[j] {
                if delta[i].m.determinant() < 0.0 {
                    problem.require_over(j, i);
                } else {
                    problem.require_over(i, j);
                }
            }
        }
    }
    // Transitivity where three faces share a region.
    for i in 0..n {
        for j in i + 1..n {
            if !overlaps[i][j] {
                continue;
            }
            let ij = clip(&images[i], &images[j]);
            for k in j + 1..n {
                if overlaps[i][k] && overlaps[j][k] && area(&clip(&ij, &images[k])) > 1e-9 {
                    problem.transitive(i, j, k);
                }
            }
        }
    }
    // Taco-tortilla: a face across a folded edge; a face across a flat joint.
    for (f, g, a, b) in edges.iter().chain(&joints) {
        for (h, image) in images.iter().enumerate() {
            if h != *f && h != *g && through(image, *a, *b) {
                problem.not_between(h, *f, *g);
            }
        }
    }
    // Along one line: two folds on the same side (taco-taco), a joint and a fold (taco-tortilla),
    // two joints (tortilla-tortilla).
    let side = |f: usize, a: DVec2, b: DVec2| cross(b - a, after[f].centroid_image() - a) > 0.0;
    let along = |(a1, b1): (DVec2, DVec2), (a2, b2): (DVec2, DVec2)| {
        let d = (b1 - a1).normalize_or_zero();
        if cross(d, a2 - a1).abs() > 1e-7 || cross(d, b2 - a1).abs() > 1e-7 {
            return false;
        }
        let (s2, t2) = ((a2 - a1).dot(d), (b2 - a1).dot(d));
        let (lo, hi) = (s2.min(t2).max(0.0), s2.max(t2).min((b1 - a1).dot(d)));
        hi - lo > 1e-6
    };
    for (e, (f1, g1, a1, b1)) in edges.iter().enumerate() {
        for (f2, g2, a2, b2) in &edges[e + 1..] {
            if [f2, g2].iter().any(|x| *x == f1 || *x == g1) {
                continue;
            }
            if along((*a1, *b1), (*a2, *b2)) && side(*f1, *a1, *b1) == side(*f2, *a1, *b1) {
                problem.not_interleaved((*f1, *g1), (*f2, *g2));
            }
        }
        for (h, k, a2, b2) in &joints {
            if along((*a1, *b1), (*a2, *b2)) {
                let near = if side(*h, *a1, *b1) == side(*f1, *a1, *b1) {
                    *h
                } else {
                    *k
                };
                if near != *f1 && near != *g1 {
                    problem.not_between(near, *f1, *g1);
                }
            }
        }
    }
    for (e, (f1, g1, a1, b1)) in joints.iter().enumerate() {
        for (f2, g2, a2, b2) in &joints[e + 1..] {
            if [f2, g2].iter().any(|x| *x == f1 || *x == g1) || !along((*a1, *b1), (*a2, *b2)) {
                continue;
            }
            // f2 on the side of f1.
            let (f2, g2) = if side(*f2, *a1, *b1) == side(*f1, *a1, *b1) {
                (*f2, *g2)
            } else {
                (*g2, *f2)
            };
            problem.same_order((*f1, f2), (*g1, g2));
        }
    }
    // Where nothing decides: a face that moves keeps its old order with one that stays, the order
    // of their layers even where they did not overlap (it moves in front of the paper or behind,
    // or slides into a pocket between two layers, never through them); the folding motion orders
    // the rest.
    let moves: Vec<bool> = delta.iter().map(|d| !d.close_to(&identity)).collect();
    let prefer = |i: usize, j: usize| {
        let (fi, fj) = (&before.faces[i], &before.faces[j]);
        if moves[i] != moves[j] && fi.layer != fj.layer {
            fi.layer > fj.layer
        } else {
            hint[i] >= hint[j]
        }
    };
    let Some(above) = problem.solve(&prefer) else {
        log::warn!("layers: no order found, the folding motion decides");
        let mut layers: Vec<(usize, i32)> = hint.iter().copied().enumerate().collect();
        layers.sort_by_key(|(_, h)| *h);
        let mut out = vec![0; n];
        for (layer, (i, _)) in layers.into_iter().enumerate() {
            out[i] = layer as i32;
        }
        return out;
    };
    // Bottom up: a face goes once every face it is over is placed; the hint picks among the
    // ready ones, and breaks a cycle (faces over each other in different places).
    let mut placed = vec![false; n];
    let mut layers = vec![0; n];
    for layer in 0..n {
        let ready = (0..n)
            .filter(|i| !placed[*i])
            .filter(|i| (0..n).all(|j| placed[j] || !above[*i][j]))
            .min_by_key(|i| hint[*i])
            .or_else(|| (0..n).filter(|i| !placed[*i]).min_by_key(|i| hint[*i]))
            .expect("a face left");
        placed[ready] = true;
        layers[ready] = layer as i32;
    }
    layers
}

/// Works out a collapse, see [`Collapse`].
fn plan_collapse(paper: &Paper, fold: Fold, collapse: Collapse, step: usize) -> Result<Planned, FoldError> {
    let folds: Vec<(DVec2, DVec2, FoldKind)> = collapse
        .fold
        .iter()
        .map(|(a, b, k)| {
            let k = if collapse.mirror { k.opposite() } else { *k };
            (collapse.point(*a), collapse.point(*b), k)
        })
        .collect();
    let flats: Vec<(DVec2, DVec2)> = collapse
        .flat
        .iter()
        .map(|(a, b)| (collapse.point(*a), collapse.point(*b)))
        .collect();
    // The faces cut along the creases that fold.
    let mut faces = paper.faces.clone();
    for (a, b, _) in &folds {
        let d = (*b - *a).normalize_or_zero();
        let mut cut = Vec::new();
        for face in faces {
            if crosses(&face, *a, *b) {
                let (l, r) = split_paper(&face, *a, d);
                cut.extend(l);
                cut.extend(r);
            } else {
                cut.push(face);
            }
        }
        faces = cut;
    }
    let before = Paper {
        faces,
        creases: paper.creases.clone(),
    };
    let n = before.faces.len();
    // The hinges between faces: how the second face sits on the first, and the kind of a new fold.
    let mut hinges: Vec<Vec<(usize, Pose, Option<FoldKind>)>> = vec![Vec::new(); n];
    for (i, hinges_i) in hinges.iter_mut().enumerate() {
        for j in 0..n {
            if i == j {
                continue;
            }
            let (f, g) = (&before.faces[i], &before.faces[j]);
            for (p, q) in shared_edges(f, g) {
                let new_fold = folds.iter().find(|(a, b, _)| on_segment((*a, *b), p, q));
                let opens = flats.iter().any(|(a, b)| on_segment((*a, *b), p, q));
                let reflect = Pose::reflection(p, (q - p).normalize());
                let relation = if let Some((_, _, kind)) = new_fold {
                    // As the viewer sees it from the face that stays: past a valley the paper comes
                    // to the front, past a mountain it goes to the back.
                    let seen = if f.pose.flipped() { kind.opposite() } else { *kind };
                    (reflect, Some(seen))
                } else if opens || f.pose.close_to(&g.pose) {
                    (Pose::IDENTITY, None)
                } else {
                    (reflect, None)
                };
                hinges_i.push((j, relation.0, relation.1));
            }
        }
    }
    // The walk from the face that stays.
    let grip = match fold.moving {
        // Already mirrored with the fold.
        Grip::Paper(p) => p,
        Grip::Top(q) => top_at(paper, q).ok_or(FoldError::PointOffPaper(q))?,
    };
    let root = before
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.contains(grip))
        .max_by_key(|(_, f)| f.layer)
        .map(|(i, _)| i)
        .ok_or(FoldError::PointOffPaper(grip))?;
    let mut poses: Vec<Option<Pose>> = vec![None; n];
    // How far from the faces that stay, and toward the viewer or away.
    let mut depth = vec![0usize; n];
    let mut toward = vec![FoldKind::Valley; n];
    poses[root] = Some(before.faces[root].pose);
    let mut queue = std::collections::VecDeque::from([root]);
    while let Some(i) = queue.pop_front() {
        let pose = poses[i].expect("placed");
        let moved = !pose.close_to(&before.faces[i].pose);
        for (j, relation, kind) in hinges[i].clone() {
            let placed = relation.then(&pose);
            match poses[j] {
                Some(p) if !p.close_to(&placed) => {
                    return Err(FoldError::Inconsistent(before.faces[j].centroid()));
                }
                Some(_) => {}
                None => {
                    poses[j] = Some(placed);
                    let j_moves = !placed.close_to(&before.faces[j].pose);
                    if j_moves {
                        depth[j] = if moved { depth[i] + 1 } else { 1 };
                        toward[j] = if moved {
                            toward[i]
                        } else {
                            kind.unwrap_or(FoldKind::Valley)
                        };
                    }
                    queue.push_back(j);
                }
            }
        }
    }
    let moving: Vec<bool> = (0..n)
        .map(|i| poses[i].is_some_and(|p| !p.close_to(&before.faces[i].pose)))
        .collect();
    if !moving.iter().any(|m| *m) {
        return Err(FoldError::NothingMoves);
    }
    // The layers: the faces that stay keep theirs; the ones that come to the front go on top, the
    // nearest to the faces that stay highest (they cover what folds behind them), the ones that go
    // to the back under everything, the nearest lowest.
    let (lo, hi) = (
        before.faces.iter().map(|f| f.layer).min().unwrap_or(0),
        before.faces.iter().map(|f| f.layer).max().unwrap_or(0),
    );
    let max_depth = depth.iter().copied().max().unwrap_or(0) as i32 + 1;
    let span = hi - lo + 1;
    let mut landed = before.clone();
    for (i, face) in landed.faces.iter_mut().enumerate() {
        face.pose = poses[i].unwrap_or(face.pose);
        let layer = face.layer - lo;
        face.layer = if !moving[i] {
            span * max_depth + layer
        } else {
            let d = depth[i] as i32;
            match toward[i] {
                FoldKind::Valley => span * (2 * max_depth - d + 1) + (span - 1 - layer),
                FoldKind::Mountain => span * (d - 1) + (span - 1 - layer),
            }
        };
    }
    compact_layers(&mut landed);
    let kinds = |p: DVec2, q: DVec2| {
        folds
            .iter()
            .find(|(a, b, _)| on_segment((*a, *b), p, q))
            .map(|(_, _, k)| *k)
            .or_else(|| {
                paper
                    .creases
                    .iter()
                    .find(|c| on_segment((c.a, c.b), p, q))
                    .map(|c| c.kind)
            })
    };
    let hint: Vec<i32> = landed.faces.iter().map(|f| f.layer).collect();
    let layers = stack(&before, &landed.faces, &kinds, &hint);
    let top = layers.iter().copied().max().unwrap_or(0);
    for (i, (face, layer)) in landed.faces.iter_mut().zip(layers).enumerate() {
        face.layer = layer;
        // The faces that end in the upper half of the stack come over the paper on their way.
        toward[i] = if 2 * layer >= top {
            FoldKind::Valley
        } else {
            FoldKind::Mountain
        };
    }
    // The new creases, and the fold lines to draw: on every face edge along them.
    let mut creases = paper.creases.clone();
    for (a, b, kind) in &folds {
        creases.retain(|c| !same_segment((c.a, c.b), (*a, *b)));
        creases.push(Crease {
            a: *a,
            b: *b,
            kind: *kind,
            step,
        });
    }
    let mut chords = Vec::new();
    for (i, face) in before.faces.iter().enumerate() {
        let edge_count = face.poly.len();
        for e in 0..edge_count {
            let (p, q) = (face.poly[e], face.poly[(e + 1) % edge_count]);
            if let Some((_, _, kind)) = folds.iter().find(|(a, b, _)| on_segment((*a, *b), p, q)) {
                chords.push(Chord {
                    face: i,
                    a: p,
                    b: q,
                    kind: *kind,
                });
            }
        }
    }
    landed.creases = creases;
    // The arrow follows the biggest face that moves.
    let arrow = (0..n)
        .filter(|i| moving[*i])
        .max_by(|a, b| before.faces[*a].area().total_cmp(&before.faces[*b].area()))
        .map(|i| before.faces[i].centroid())
        .unwrap_or(grip);
    let axis = folds
        .first()
        .map(|(a, b, _)| {
            let pose = before.faces[before.face_at(*a).unwrap_or(root)].pose;
            (pose.apply(*a), (pose.m * (*b - *a)).normalize_or_zero())
        })
        .unwrap_or((DVec2::ZERO, DVec2::X));
    Ok(Planned {
        fold,
        turns: toward,
        before,
        moving,
        grip: arrow,
        axis,
        side: DVec2::new(-axis.1.y, axis.1.x),
        after: landed.clone(),
        landed,
        chords,
        reverse: None,
    })
}

fn same_segment(p: (DVec2, DVec2), q: (DVec2, DVec2)) -> bool {
    (p.0.distance(q.0) < 1e-6 && p.1.distance(q.1) < 1e-6) || (p.0.distance(q.1) < 1e-6 && p.1.distance(q.0) < 1e-6)
}

/// The layers renumbered from 0 without gaps, the faces of a layer stay together.
fn compact_layers(paper: &mut Paper) {
    let mut layers: Vec<i32> = paper.faces.iter().map(|f| f.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    for face in &mut paper.faces {
        face.layer = layers.binary_search(&face.layer).unwrap_or(0) as i32;
    }
}

/// Plans every step of a sequence, each one on the paper the previous one left.
pub fn plan_all(start: Paper, folds: &[Fold]) -> Result<(Vec<Planned>, Paper), (usize, FoldError)> {
    let mut paper = start;
    let mut plans = Vec::new();
    for (i, fold) in folds.iter().enumerate() {
        let planned = plan(&paper, *fold, i).map_err(|e| (i, e))?;
        paper = planned.after.clone();
        plans.push(planned);
    }
    Ok((plans, paper))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BL: DVec2 = DVec2::new(-1.0, -1.0);
    const BR: DVec2 = DVec2::new(1.0, -1.0);
    const TR: DVec2 = DVec2::new(1.0, 1.0);
    const TL: DVec2 = DVec2::new(-1.0, 1.0);

    fn close(a: DVec2, b: DVec2) -> bool {
        a.distance(b) < 1e-9
    }

    #[test]
    fn diagonal_valley_fold() {
        let paper = Paper::square(Pose::IDENTITY);
        let planned = plan(&paper, Fold::bring(BL, TR), 0).unwrap();
        assert_eq!(planned.before.faces.len(), 2);
        assert_eq!(planned.moving.iter().filter(|m| **m).count(), 1);
        assert!(close(planned.after.image_of(BL).unwrap(), TR));
        // The folded half is on top and shows its back.
        let folded = planned.after.faces.iter().find(|f| f.contains(BL)).unwrap();
        assert!(folded.pose.flipped());
        assert_eq!(folded.layer, 1);
        assert_eq!(planned.after.creases.len(), 1);
        assert_eq!(planned.after.creases[0].kind, FoldKind::Valley);
        let total: f64 = planned.after.faces.iter().map(Face::area).sum();
        assert!((total - 4.0).abs() < 1e-9);
    }

    #[test]
    fn mountain_goes_under() {
        let paper = Paper::square(Pose::IDENTITY);
        let planned = plan(&paper, Fold::mountain(Line::Bring(BL, TR), BL), 0).unwrap();
        let folded = planned.after.faces.iter().find(|f| f.contains(BL)).unwrap();
        assert_eq!(folded.layer, 0);
        assert_eq!(planned.after.creases[0].kind, FoldKind::Mountain);
    }

    #[test]
    fn precrease_leaves_the_paper_flat() {
        let paper = Paper::square(Pose::IDENTITY);
        let planned = plan(&paper, Fold::bring(TL, BR).and_unfold(), 0).unwrap();
        assert!(planned.after.faces.iter().all(|f| f.pose == Pose::IDENTITY));
        assert_eq!(planned.after.creases.len(), 1);
        // Folding along the precrease again records the fold, though no face is cut.
        let again = plan(&planned.after, Fold::bring(TL, BR), 1).unwrap();
        assert_eq!(again.chords.len(), 1);
        assert_eq!(again.after.creases.len(), 1);
        assert_eq!(again.after.creases[0].step, 1);
    }

    #[test]
    fn flap_fold_moves_one_layer() {
        let paper = Paper::square(Pose::IDENTITY);
        let triangle = plan(&paper, Fold::bring(BL, TR), 0).unwrap().after;
        // Both corners are at the top right, only the front one comes down.
        let line = Line::Through(DVec2::new(0.5, 1.0), DVec2::new(1.0, 0.5));
        let flap = plan(&triangle, Fold::valley(line, BL).flap(), 1).unwrap();
        assert_eq!(flap.moving.iter().filter(|m| **m).count(), 1);
        assert!(close(flap.after.image_of(TR).unwrap(), TR));
        assert!(!close(flap.after.image_of(BL).unwrap(), TR));
        let all = plan(&triangle, Fold::valley(line, BL), 1).unwrap();
        assert_eq!(all.moving.iter().filter(|m| **m).count(), 2);
        assert!(close(all.after.image_of(BL).unwrap(), all.after.image_of(TR).unwrap()));
    }

    #[test]
    fn edge_onto_edge_makes_the_kite() {
        let paper = Paper::square(Pose::IDENTITY);
        let fold = Fold::valley(Line::EdgeOnto(BL, TL, BL, TR), TL);
        let planned = plan(&paper, fold, 0).unwrap();
        let tl = planned.after.image_of(TL).unwrap();
        // TL lands on the diagonal, at its distance from BL.
        assert!(cross(TR - BL, tl - BL).abs() < 1e-9);
        assert!((tl.distance(BL) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn inside_reverse_reverses_half_the_spine() {
        let paper = Paper::square(Pose::IDENTITY);
        let rectangle = plan(&paper, Fold::bring(TL, BL), 0).unwrap().after;
        let fold = Fold::inside_reverse(Line::Through(DVec2::ZERO, BL), DVec2::new(-1.0, 0.0));
        let planned = plan(&rectangle, fold, 1).unwrap();
        // The corner lands inside, between the two layers that stay.
        let corner = planned.after.image_of(DVec2::new(-1.0, 0.0)).unwrap();
        assert!(corner.distance(DVec2::new(0.0, -1.0)) < 1e-9);
        let spine: Vec<_> = planned
            .after
            .creases
            .iter()
            .filter(|c| c.a.y == 0.0 && c.b.y == 0.0)
            .collect();
        assert_eq!(spine.len(), 2);
        assert_ne!(spine[0].kind, spine[1].kind);
        let layers: Vec<i32> = planned.after.faces.iter().map(|f| f.layer).collect();
        let (lo, hi) = (*layers.iter().min().unwrap(), *layers.iter().max().unwrap());
        for (face, m) in planned.after.faces.iter().zip(&planned.moving) {
            if *m {
                assert!(face.layer > lo && face.layer < hi, "the reversed flap is inside");
            }
        }
    }

    #[test]
    fn turn_over_mirrors_and_restacks() {
        let paper = Paper::square(Pose::IDENTITY);
        let triangle = plan(&paper, Fold::bring(BL, TR), 0).unwrap().after;
        let over = plan(&triangle, Fold::turn_over(), 1).unwrap();
        assert!(over.after.image_of(TR).unwrap().distance(DVec2::new(-1.0, 1.0)) < 1e-9);
        let front = over
            .after
            .faces
            .iter()
            .find(|f| f.contains(TR) && f.contains(TL))
            .unwrap();
        assert_eq!(front.layer, 1);
        assert!(front.pose.flipped());
    }

    #[test]
    fn off_paper_points_are_errors() {
        let paper = Paper::square(Pose::IDENTITY);
        let fold = Fold::bring(DVec2::new(3.0, 3.0), TR);
        assert_eq!(
            plan(&paper, fold, 0),
            Err(FoldError::PointOffPaper(DVec2::new(3.0, 3.0)))
        );
        assert_eq!(plan(&paper, Fold::bring(BL, BL), 0), Err(FoldError::DegenerateLine));
    }
}
