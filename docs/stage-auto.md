# The auto stage

`stage = light` (`#E5E5E5`) and `stage = dark` (`#14213D`) are fixed. `stage = auto`, **the default**, picks the stage
for each picture of the documentation (the Anatomy and every row of the variant axes) with this rule; the
Spacing uses the stage the Anatomy got:

1. **Effective background.** The solid fill of the component's root; if it has none (or it is not a
   solid), the first ancestor that has one; if none does, the component is transparent.
   The host's `node.get` gives no parent, so the only ancestor known is the **component set** of a
   variant. A component that is not in a set has no ancestor to look at.
2. **What is judged.**
   - with a background: that colour (it is what meets the stage; the elements sit on it);
   - without: the colours of the visible elements under the root (text, icons, strokes and fills).
   A hidden layer, a hidden paint and a paint with opacity 0 don't count. A colour with transparency
   is composited over the stage it is tried on.
3. **Choice.** For the light stage (`#E5E5E5`) and the dark stage (`#14213D`) take the **lowest WCAG
   contrast** (`(L1 + 0.05) / (L2 + 0.05)`) between those colours and the stage; the stage with the
   higher lowest contrast wins. **A tie is light**; nothing to judge is light.

Examples: white text on a transparent component → dark; dark text on a transparent component →
light; white text on a blue button → light (the blue is judged: 4.9:1 against light, 2.6:1 against
dark); a white card → dark (it separates better from `#14213D`).

The colours of the lines, markers and measures are fixed by the stage that is chosen (light: `#C54600` and
`#007D00`; dark: `#FCA311` and `#4ADE80`); they play no part in the choice.

## Room for effects

A stage is clipped. Its height is the instance plus the room the instance's shadows and blurs need, so they are
not cut: a drop shadow reaches its radius plus its spread, moved by its offset; a layer blur reaches its radius; an
inner shadow and a background blur stay inside the box. Only visible effects count, and an effect of a layer inside
the instance counts only for what passes the instance's box. The room is the largest reach plus 8 px, at most 400 px
per side, and never less than the room a stage always had (96 px above and below in the Anatomy and the Spacing, 32 px
in a variant row). Each variant row uses its own variant's effects. The width is not changed: a row that would need
more is listed in `effects.narrow` of the run's output. A variant row's stage is 42 px narrower than the Anatomy's (558 px when the Anatomy's is 600 px, measured): the row is a block with a 20 px padding and a 1 px border (`row_stage_w` in `draw_axes`, from `BLOCK_PAD` and `BLOCK_BORDER`).

Code: `src/contrast.rs` (`pick_stage`) and `effective_backdrop` / `is_dark` in `src/gerar.rs`;
tests in both.
