# Bevygami

Learn origami step by step, a game made with [Bevy](https://bevyengine.org) 0.20 and its feathers
widgets. The paper folds in 3D: drag the flap that glows, turn the sheet around with the mouse or
a finger, go back and forth through the steps. It runs on Linux, Windows, Android and in the
browser.

## Lessons

| Lesson               | Teaches                                                         |
| -------------------- | --------------------------------------------------------------- |
| 1. The diagonal fold | the valley fold, folding all the layers                         |
| 2. The mountain fold | the mountain fold, the paper going behind                       |
| 3. Fold and unfold   | precreases, the book fold                                       |
| 4. The kite base     | bringing an edge onto a crease (Huzita–Hatori axiom O3)         |
| 5. The blintz        | corners to the center (axiom O2)                                |
| 6. Pleats            | valley and mountain side by side, stacked in levels             |
| 7. The paper cup     | a traditional model, folding one flap only                      |
| 8. The dog face      | a traditional model, the first of many folders                  |
| 9. The waterbomb base | the inside reverse fold                                        |
| 10. The preliminary base | inside reverse folds from a triangle, the start of the crane |
| 11. The bird base    | precreases, turning over, the petal folds (collapses)            |
| 12. The crane        | the orizuru: legs narrowed, neck and tail reversed, head, wings  |

The traditional models and the preliminary base start white side up, so the color ends outside.

## How the lines read

The game follows the conventions of the diagrams (Yoshizawa–Randlett) and of the crease pattern
editors (ORIPA, Oriedita, Box Pleating Studio):

- **Valley fold**: the paper comes toward you; a dashed line, blue.
- **Mountain fold**: the paper goes behind; a dash-dot-dot line, red.
- The fold of the current step **breathes**, and so does the flap that will move. The creases of
  the steps done are thin solid lines, **fainter the older they are**.
- The **arrow** shows how the paper moves (a full head for a valley, a half head for a mountain,
  heads at both ends to fold and unfold), the **dotted outline** where the flap lands (the X-ray
  line of the diagrams).
- The **crease pattern** in the corner is the sheet unfolded, front side up: the creases made so
  far, the fold to come, and the part of the sheet that moves, lightened.

The line pattern always comes with the color, the color is never the only cue. The crease colors
were checked with a palette validator (lightness, chroma, color-blind separation and contrast
≥ 3:1) on every paper color and on the light and dark surfaces.

## Controls

- Drag the glowing flap toward the arrow, past half way, and let go: it folds. A tap on the flap,
  **Fold**, → Space or Enter fold it for you; **‹ Back**, ← or Backspace unfold the last step.
- Drag around the paper (or with the right button) to turn it, the wheel or a pinch to zoom, R or
  **Reset view** to come back.
- The cursor says what a press does: an open hand over the flap (or a little around it), a closed
  hand while it folds, two arrows around a circle where a drag turns the model.
- Ctrl and the wheel, Ctrl+= Ctrl+- Ctrl+0, a pinch, or the buttons of the settings zoom the
  text and the panels, for a display that is hard to read.
- **Lessons** (a feathers menu, with the step reached and the lessons done), **Settings** (dark
  or light theme, the paper color, thin, normal or bold lines, the zoom, the arrows, the landing
  outline, the crease pattern, reduce motion), **?** for the help. At the end of a lesson,
  **Next lesson ›**.

Each lesson opens again at the step it was left. The settings and the progress are saved in `~/.config/bevygami/settings.json` (`%APPDATA%` on
Windows, the `localStorage` in the browser).

## The folding engine

`src/fold.rs` has no Bevy rendering in it. The sheet is a set of flat convex faces, each with its
polygon in the coordinates of the unfolded square, an isometry to the folded model (reflections
included: the back of the paper up), and a layer. A step is a fold line built from points of the
paper like the Huzita–Hatori axioms (O1 through two points, O2 a point onto a point, O3 an edge
onto an edge), a point that moves, valley or mountain, all the layers or one flap (the faces on the
moving side joined to the moving point through the paper), and fold or fold-and-unfold. The
moving point can also be the top layer at a point of the folded model, and the line two points
of the folded model, for the steps on many layers. Inside and outside reverse folds turn the two
halves of a flap opposite ways and reverse the part of its spine that moves; a turn over mirrors
the whole model and its layers.

A reverse fold is taught as by hand: the flap folded along the line through its layers and
unfolded, the crease marks it, then the reverse fold. Its motion is rigid (`src/motion.rs`): the
two halves of the flap are joined along the spine, turned each around the line they would tear.
The paper moves like a vertex of four creases (the spine of the flap, the fold line on its two
halves, the spine of what stays): the front layers lift around their spine, flat when the spine of
the flap starts on it, the spine of the flap swings between the layers from where it was to where
it lands, and the layers close. The halves of the flap are the upper and lower half of its own
layers (the crane's legs are eight layers in a zigzag, joined front to back along one spine), and
the layers that open are found through the paper: those joined to the flap, and on wherever they
are joined away from the line they open around; the rest does not move. A test checks every edge
stays joined along the motion.

Where stiff paper cannot do it, the paper **bends** (`src/bend.rs`): the crane's tail and head,
where opening the body flat would unfold the neck again. The layers next to the flap open a little
(40° at most), the spine of the flap swings exactly, and wherever two faces the paper joins come
apart, the edge goes to the middle of the two and the difference fades into each face: the paper
flexes near the edge, as in the hands. The faces of those steps are meshed finely with the same
points on both sides of every edge, so nothing tears; a test checks it. The collapses bend too:
each face first goes from where it was to where it lands on its own, then the faces that move are
brought together, each refitted as a rigid piece onto the middle of where it and its neighbors put
its edges (200 rounds a frame), and what is left apart bends. The petal folds go from 0.9 apart
to 0.05; the leg narrowings fold over in one piece and do not need it.

A **collapse** folds several creases at once, as the CP editors fold a crease pattern (the petal
folds of the bird base, the leg narrowings of the crane): the step lists the creases that fold
(mountain or valley from the front side) and the ones that open flat, every other crease keeps its
state. The faces are placed by walking the paper from a face that stays and reflecting across
every folded crease crossed; two ways around the paper that disagree make the step an error, so a
wrong crease pattern cannot fold. Each face moves from where it was to where it lands as a screw
in 3D: a half turn around the axis of its reflection, or a turn in the plane lifted over the paper.

The layers of a collapse come from a **layer solver** (`src/layers.rs`), as in ORIPA and Oriedita:
one unknown per pair of overlapping faces, and the conditions of paper. Across a valley the front
sides of the two faces face each other (the back sides across a mountain). Faces that do not move
keep their order. Three faces over one region are ordered transitively. A face across a folded
edge is not between the two faces of the fold (taco-tortilla), two folds along one edge are nested
or apart (taco-taco), and the same for the creases left flat (tortillas). A search with unit
propagation solves them; where nothing decides, a face that moves keeps its old order with one
that stays (it goes in front of the paper, behind it or into a pocket, not through it). The
narrowing of a leg shows why it matters: its line goes on through the flat joint at the hinge into
the corner of the petal, six creases around a kite tip, and the corner of the petal folds over
with the leg. The bird base and the crane check flat-foldable at every vertex (tests). The faces
are cut along the line, the moving ones reflected and restacked (on top for a valley, under for a
mountain, their order reversed); in 3D the reflection is a half turn around the line, which is the
animation. The creases are recorded mountain or valley as seen from the front side, as in ORIPA.

`src/cp.rs` checks the crease patterns locally: Maekawa (`|M − V| = 2`) and Kawasaki (alternate
angles sum to π) at every inner vertex, the ground for the crease pattern lessons.

## Roadmap

1. **Now**: simple folds (valley, mountain, fold and unfold, flaps), inside and outside reverse
   folds, turning over, collapses (the petal fold); the bases and models above, up to the crane.
2. **Compound folds as collapses** (the petal fold is one): squash, rabbit ear and swivel folds,
   sinks; the frog, the lily, the flapping bird.
3. **Folding from a whole crease pattern**: the collapse already places the faces from the CP
   (ORIPA's folded estimation) and stacks them with the layer solver; next, the simple folds on
   the solver too, and multi-level pleats collapsed from their CP.
4. **Design**: Robert J. Lang's tree method (*Origami Design Secrets*, TreeMaker) and box pleating
   on a grid (Box Pleating Studio): a stick figure in, a crease pattern out, folded in 3D.

The code is written from the published conventions and algorithms, not ported from those programs
(ORIPA is GPL-3.0-or-later).

## Building

```sh
cargo dev                              # local runs, Bevy as a shared library (fast links with mold)
cargo build --release                  # self-contained binary, for the other machines
cargo test                             # the folding engine and every lesson
trunk serve                            # the web version, http://127.0.0.1:8080
cd mobile && cargo apk build --release --lib   # Android
```

`.cargo/config.toml` links with clang and mold on Linux. The `dynamic` feature (`cargo dev`) is
only for the local runs: a binary built with it needs `libbevy_dylib.so` next to it.

Screenshots without a person at the mouse: `BEVYGAMI_SCREENSHOT=out.png` with `BEVYGAMI_LESSON=n`,
`BEVYGAMI_STEP=n`, `BEVYGAMI_T=0.5` (a fold half way), `BEVYGAMI_YAW` / `BEVYGAMI_PITCH`,
`BEVYGAMI_THEME=light`, `BEVYGAMI_PANEL=settings|help`, `BEVYGAMI_ZOOM=1.25`, `BEVYGAMI_LINES=bold`, `BEVYGAMI_FRESH=1` (default settings, not
saved).

CI: `.github/workflows/builds.yml` (tests, clippy, Linux, Windows, Android; a release for the
`v*` tags), `.github/workflows/pages.yml` (the web version on GitHub Pages).

## License

MIT
