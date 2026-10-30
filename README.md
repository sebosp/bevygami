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
- **Lessons** (a feathers menu), **Settings** (dark or light theme, the paper color, the arrows,
  the landing outline, the crease pattern, reduce motion), **?** for the help.

The settings and the step reached are saved in `~/.config/bevygami/settings.json` (`%APPDATA%` on
Windows, the `localStorage` in the browser).

## The folding engine

`src/fold.rs` has no Bevy rendering in it. The sheet is a set of flat convex faces, each with its
polygon in the coordinates of the unfolded square, an isometry to the folded model (reflections
included: the back of the paper up), and a layer. A step is a fold line built from points of the
paper like the Huzita–Hatori axioms (O1 through two points, O2 a point onto a point, O3 an edge
onto an edge), a point that moves, valley or mountain, all the layers or one flap (the faces on the
moving side joined to the moving point through the paper), and fold or fold-and-unfold. The faces
are cut along the line, the moving ones reflected and restacked (on top for a valley, under for a
mountain, their order reversed); in 3D the reflection is a half turn around the line, which is the
animation. The creases are recorded mountain or valley as seen from the front side, as in ORIPA.

`src/cp.rs` checks the crease patterns locally: Maekawa (`|M − V| = 2`) and Kawasaki (alternate
angles sum to π) at every inner vertex, the ground for the crease pattern lessons.

## Roadmap

1. **Now**: simple folds (valley, mountain, fold and unfold, flaps), the traditional bases and
   models above.
2. **Compound folds**: inside and outside reverse folds, squash, petal and rabbit ear folds, swivel
   folds. They fold several creases at once: the waterbomb and preliminary bases, the bird base,
   the crane.
3. **Folding from a crease pattern**: the flat folded shape from the CP by reflecting the faces
   along a spanning tree (ORIPA's folded estimation), then the layer order by constraint solving
   (the face-pair and the 3- and 4-face constraints of Jason Ku and of ORIPA). Then collapses of
   multi-level pleats.
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
`BEVYGAMI_THEME=light`, `BEVYGAMI_PANEL=settings|help`, `BEVYGAMI_FRESH=1` (default settings, not
saved).

CI: `.github/workflows/builds.yml` (tests, clippy, Linux, Windows, Android; a release for the
`v*` tags), `.github/workflows/pages.yml` (the web version on GitHub Pages).

## License

MIT
