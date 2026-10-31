//! The order of the layers of a flat folded state, found as the CP editors find it (Jun Mitani's
//! ORIPA, Oriedita): one unknown per pair of overlapping faces, which of the two is over the other,
//! and the conditions a sheet of paper puts on them, solved by a search.
//!
//! The conditions:
//! - the order of two faces joined by a folded crease, from its kind;
//! - the orders that cannot change, given by the caller (faces that do not move);
//! - transitivity, where three faces overlap in one region;
//! - taco-tortilla: a face that covers a folded edge is not between the two faces of the fold;
//! - taco-taco: two folds along the same edge, on the same side, are nested or apart, never
//!   interleaved;
//! - tortillas: the faces here are also cut along the creases left flat, a face across such a
//!   joint is on the same side of both halves, two joints along one line keep their order on
//!   both sides, and a joint along a folded edge is not between the two faces of the fold.
//!
//! The search follows a preference (the order the folding motion suggests) and goes back on a
//! contradiction, a DPLL search with unit propagation.
use bevy::math::DVec2;

use crate::fold::cross;

/// A literal: the pair's unknown and the value that makes it true.
type Lit = (usize, bool);

pub struct Problem {
    n: usize,
    /// The unknown of each overlapping pair `(i, j)`, `i < j`, true when `i` is over `j`.
    var: Vec<Vec<Option<usize>>>,
    pairs: Vec<(usize, usize)>,
    clauses: Vec<Vec<Lit>>,
    /// The orders known before the search, they may contradict each other.
    units: Vec<Lit>,
}

impl Problem {
    pub fn new(overlaps: &[Vec<bool>]) -> Problem {
        let n = overlaps.len();
        let mut var = vec![vec![None; n]; n];
        let mut pairs = Vec::new();
        for i in 0..n {
            for j in i + 1..n {
                if overlaps[i][j] {
                    var[i][j] = Some(pairs.len());
                    pairs.push((i, j));
                }
            }
        }
        Problem {
            n,
            var,
            pairs,
            clauses: Vec::new(),
            units: Vec::new(),
        }
    }

    /// "`i` is over `j`" as a literal, when the two overlap.
    pub fn over(&self, i: usize, j: usize) -> Option<Lit> {
        if i < j {
            self.var[i][j].map(|v| (v, true))
        } else {
            self.var[j][i].map(|v| (v, false))
        }
    }

    pub fn require_over(&mut self, i: usize, j: usize) {
        if let Some(lit) = self.over(i, j) {
            self.units.push(lit);
        }
    }

    /// No cycle among three faces that overlap in one region.
    pub fn transitive(&mut self, i: usize, j: usize, k: usize) {
        let (Some(ij), Some(jk), Some(ik)) = (self.over(i, j), self.over(j, k), self.over(i, k)) else {
            return;
        };
        // i > j > k implies i > k, and the other way round.
        self.clauses.push(vec![not(ij), not(jk), ik]);
        self.clauses.push(vec![ij, jk, not(ik)]);
    }

    /// `h` is on the same side of `f` and of `g`.
    pub fn not_between(&mut self, h: usize, f: usize, g: usize) {
        let (Some(hf), Some(hg)) = (self.over(h, f), self.over(h, g)) else {
            return;
        };
        self.clauses.push(vec![not(hf), hg]);
        self.clauses.push(vec![hf, not(hg)]);
    }

    /// `a` is over `b` exactly when `c` is over `d`.
    pub fn same_order(&mut self, (a, b): (usize, usize), (c, d): (usize, usize)) {
        let (Some(ab), Some(cd)) = (self.over(a, b), self.over(c, d)) else {
            return;
        };
        self.clauses.push(vec![not(ab), cd]);
        self.clauses.push(vec![ab, not(cd)]);
    }

    /// The folds `f1`-`g1` and `f2`-`g2` along one edge are not interleaved: `f2` and `g2` are both
    /// between `f1` and `g1`, or neither is.
    pub fn not_interleaved(&mut self, (f1, g1): (usize, usize), (f2, g2): (usize, usize)) {
        let lits = [
            self.over(f2, f1),
            self.over(f2, g1),
            self.over(g2, f1),
            self.over(g2, g1),
        ];
        let [Some(a), Some(b), Some(c), Some(d)] = lits else {
            return;
        };
        // The four are true an even number of times: one clause against each odd assignment.
        let lits = [a, b, c, d];
        for mask in 0u32..16 {
            if mask.count_ones() % 2 == 1 {
                let clause = (0..4)
                    .map(|k| if mask & (1 << k) != 0 { not(lits[k]) } else { lits[k] })
                    .collect();
                self.clauses.push(clause);
            }
        }
    }

    /// Solves, following `prefer(i, j)` ("`i` over `j`") where nothing decides. `None` when the
    /// conditions contradict each other or the search takes too long.
    pub fn solve(&self, prefer: &dyn Fn(usize, usize) -> bool) -> Option<Vec<Vec<bool>>> {
        let vars = self.pairs.len();
        let mut watch: Vec<Vec<usize>> = vec![Vec::new(); vars];
        for (c, clause) in self.clauses.iter().enumerate() {
            for (v, _) in clause {
                watch[*v].push(c);
            }
        }
        let mut value: Vec<Option<bool>> = vec![None; vars];
        let mut trail: Vec<usize> = Vec::new();
        // The decisions: where they start on the trail, and whether the other value was tried.
        let mut decisions: Vec<(usize, bool)> = Vec::new();
        let mut queue: Vec<Lit> = Vec::new();
        // The units first, a contradicting unit is dropped (the hint was wrong, not the paper).
        for lit in &self.units {
            match value[lit.0] {
                None => {
                    value[lit.0] = Some(lit.1);
                    trail.push(lit.0);
                    if !propagate(&self.clauses, &watch, &mut value, &mut trail, &mut queue, *lit) {
                        return None;
                    }
                }
                Some(v) if v != lit.1 => log::debug!("layers: contradicting order dropped"),
                Some(_) => {}
            }
        }
        let mut steps = 0usize;
        let mut next = 0usize;
        loop {
            while next < vars && value[next].is_some() {
                next += 1;
            }
            if next == vars {
                break;
            }
            let (i, j) = self.pairs[next];
            let lit = (next, prefer(i, j));
            decisions.push((trail.len(), false));
            let mut ok = assign(&self.clauses, &watch, &mut value, &mut trail, &mut queue, lit);
            while !ok {
                steps += 1;
                if steps > 20_000 {
                    return None;
                }
                // Back to the last decision with another value to try.
                let start = loop {
                    let (start, tried) = decisions.pop()?;
                    if !tried {
                        break start;
                    }
                    undo(&mut value, &mut trail, start);
                };
                let v = trail[start];
                let flipped = (v, !value[v].expect("decided"));
                undo(&mut value, &mut trail, start);
                decisions.push((start, true));
                ok = assign(&self.clauses, &watch, &mut value, &mut trail, &mut queue, flipped);
                next = 0;
            }
        }
        let mut above = vec![vec![false; self.n]; self.n];
        for (v, (i, j)) in self.pairs.iter().enumerate() {
            if value[v] == Some(true) {
                above[*i][*j] = true;
            } else {
                above[*j][*i] = true;
            }
        }
        Some(above)
    }
}

fn not((v, b): Lit) -> Lit {
    (v, !b)
}

fn undo(value: &mut [Option<bool>], trail: &mut Vec<usize>, start: usize) {
    for v in trail.drain(start..) {
        value[v] = None;
    }
}

fn assign(
    clauses: &[Vec<Lit>],
    watch: &[Vec<usize>],
    value: &mut [Option<bool>],
    trail: &mut Vec<usize>,
    queue: &mut Vec<Lit>,
    lit: Lit,
) -> bool {
    value[lit.0] = Some(lit.1);
    trail.push(lit.0);
    propagate(clauses, watch, value, trail, queue, lit)
}

/// Unit propagation from a new value: false on a contradiction.
fn propagate(
    clauses: &[Vec<Lit>],
    watch: &[Vec<usize>],
    value: &mut [Option<bool>],
    trail: &mut Vec<usize>,
    queue: &mut Vec<Lit>,
    lit: Lit,
) -> bool {
    queue.clear();
    queue.push(lit);
    while let Some((v, _)) = queue.pop() {
        for &c in &watch[v] {
            let clause = &clauses[c];
            let mut open = None;
            let mut open_count = 0;
            let mut satisfied = false;
            for &(u, b) in clause {
                match value[u] {
                    Some(x) if x == b => {
                        satisfied = true;
                        break;
                    }
                    Some(_) => {}
                    None => {
                        open_count += 1;
                        open = Some((u, b));
                    }
                }
            }
            if satisfied {
                continue;
            }
            match (open_count, open) {
                (0, _) => return false,
                (1, Some((u, b))) => {
                    value[u] = Some(b);
                    trail.push(u);
                    queue.push((u, b));
                }
                _ => {}
            }
        }
    }
    true
}

/// The polygon counterclockwise.
pub fn ccw(poly: &[DVec2]) -> Vec<DVec2> {
    if crate::fold::area(poly) < 0.0 {
        poly.iter().rev().copied().collect()
    } else {
        poly.to_vec()
    }
}

/// The intersection of two convex counterclockwise polygons (Sutherland–Hodgman).
pub fn clip(subject: &[DVec2], clipper: &[DVec2]) -> Vec<DVec2> {
    let mut out = subject.to_vec();
    let n = clipper.len();
    for i in 0..n {
        let (a, b) = (clipper[i], clipper[(i + 1) % n]);
        let inside = |p: DVec2| cross(b - a, p - a) >= -1e-12;
        let input = std::mem::take(&mut out);
        let m = input.len();
        for k in 0..m {
            let (p, q) = (input[k], input[(k + 1) % m]);
            let (pin, qin) = (inside(p), inside(q));
            if pin {
                out.push(p);
            }
            if pin != qin {
                let (dp, dq) = (cross(b - a, p - a), cross(b - a, q - a));
                out.push(p + (q - p) * (dp / (dp - dq)));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// The open segment `a`-`b` goes through the inside of the convex counterclockwise polygon.
pub fn through(poly: &[DVec2], a: DVec2, b: DVec2) -> bool {
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        // Inside: strictly left of every edge, by a margin.
        let (da, db) = (cross(q - p, a - p), cross(q - p, b - p));
        let margin = 1e-7 * (q - p).length();
        let (da, db) = (da - margin, db - margin);
        if da < 0.0 && db < 0.0 {
            return false;
        }
        if da < 0.0 {
            lo = lo.max(da / (da - db));
        } else if db < 0.0 {
            hi = hi.min(da / (da - db));
        }
    }
    hi - lo > 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_in_a_cycle_are_refused() {
        let overlaps = vec![
            vec![false, true, true],
            vec![true, false, true],
            vec![true, true, false],
        ];
        let mut problem = Problem::new(&overlaps);
        problem.transitive(0, 1, 2);
        problem.require_over(0, 1);
        problem.require_over(1, 2);
        // The hint says 2 over 0, transitivity says otherwise.
        let above = problem.solve(&|i, j| i > j).unwrap();
        assert!(above[0][2]);
    }

    #[test]
    fn a_face_across_a_fold_stays_outside() {
        let overlaps = vec![vec![true; 3]; 3];
        let mut problem = Problem::new(&overlaps);
        problem.transitive(0, 1, 2);
        problem.require_over(0, 1);
        problem.not_between(2, 0, 1);
        // The hint puts 2 between 0 and 1.
        let above = problem.solve(&|i, j| [2, 0, 1][i] > [2, 0, 1][j]).unwrap();
        assert_eq!(above[2][0], above[2][1]);
    }

    #[test]
    fn clipping() {
        let square = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let moved: Vec<DVec2> = square.iter().map(|p| *p + DVec2::splat(0.5)).collect();
        let both = clip(&square, &moved);
        assert!((crate::fold::area(&both) - 0.25).abs() < 1e-12);
        assert!(through(&square, DVec2::new(-1.0, 0.5), DVec2::new(2.0, 0.5)));
        assert!(!through(&square, DVec2::new(0.0, 0.0), DVec2::new(1.0, 0.0)));
    }
}
