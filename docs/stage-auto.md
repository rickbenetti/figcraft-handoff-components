# The stage

The stage is the colour behind the component in the pictures of the documentation. There are **three** (`src/theme.rs`):

| Stage | Colour |
|---|---|
| light | `#E5E5E5` |
| mid-grey | `#777777` |
| dark | `#14213D` |

The page's **Stage** setting (`light`, `dark`, `auto`; `store.rs` `StageMode`) decides. `light` and `dark` are fixed and apply to every picture. `auto`,
**the default**, picks one of the **three** for each picture with the rule below. The panel offers Auto, Light and Dark; mid-grey is reached only by Auto.

## Where the stage is used

| Picture | What the automatic stage judges | Code |
|---|---|---|
| Component Master (the stage the set stands on) | every variant of the set (a sample of at most 30 variants and 1,200 layers in a big set) | `section.rs` `choose_stage` |
| States and Variants (the tile behind each instance) | the Master's choice | `Ctx.stage`, `tables.rs` `draw_cell` |
| Anatomy | the default variant's instance | `section.rs` `pick_stage_for`, `tables.rs` `build_anatomy` |
| Layouts (all the drawings of a component use one stage) | the default variant's instance | `spacing.rs` `build_layouts` |

There are no longer rows by variant axis with a stage of their own (0.10.0 had them): the variants are tiles of one gallery on the Master's stage.
The Master's stage is chosen whenever a run makes the Master **or** the States and Variants frame, so a "New Section" (which makes no Master) uses the
stage the page asks for.

## The rule (`src/contrast.rs`, `pick_stage3`)

1. **What is judged.** For each variant (or the one instance): the solid fill of its root, the **background**, which is what meets the stage; when the root has
   none, the solid fill of the component set; when that is missing too, the colours of the visible elements under the root (text, icons, strokes and fills).
   A hidden layer, a hidden paint and a paint with opacity 0 don't count. A colour with transparency is composited over the stage it is tried on.
2. **Choice.** For each of the three stages take the **lowest WCAG contrast** (`(L1 + 0.05) / (L2 + 0.05)`) between the colours judged and the stage; the stage with the
   **highest** lowest contrast wins. **A tie goes to light, then mid-grey, then dark**; nothing to judge is light.
3. **Warning.** When the winner's lowest contrast is under **3:1** (`MIN_STAGE_CONTRAST`, the WCAG minimum for graphics) the run raises `no_stage`
   ("No stage suits every variant: change it in Stage.").

Examples: white text on a transparent component → dark; dark text on a transparent component → light; white text on a blue button → the blue is judged
and the stage with the best lowest contrast against it wins; a component whose variants include both a white and a near-black background may have no stage
that reaches 3:1, and the run says so.

The colours of the lines, markers and measures follow the stage that is chosen and play no part in the choice: the Anatomy's markers are `#C54600` on the light stage and
`#FCA311` on the mid-grey and dark ones; the Layouts' padding is `#007D00` (light) or `#4ADE80` (dark) and its gap `#6D28D9` or `#A78BFA`.

## Room for effects

A stage is clipped. Its height is the instance plus the room the instance's shadows and blurs need, so they are not cut: a drop shadow reaches its radius plus its
spread, moved by its offset; a layer blur reaches its radius; an inner shadow and a background blur stay inside the box. Only visible effects count, and an effect of a layer
inside the instance counts only for what passes the instance's box. The room is the largest reach plus 8 px, at most 400 px per side, and never less than the room a stage
always had (96 px above and below in the Anatomy and the Layouts). The width is the frame's useful width and is not changed.

Code: `src/contrast.rs` (`pick_stage3`, `lowest_against`), `src/gerar/section.rs` (`choose_stage`, `pick_stage_for`, `judge`) and `effect_overflow` in `src/gerar.rs`; tests in each.
