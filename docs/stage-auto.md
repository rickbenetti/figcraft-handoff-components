# The auto stage

`stage = light` (the default) and `stage = dark` are fixed. `stage = auto` picks the stage for each
picture of the documentation (the Anatomy and every row of the variant axes) with this rule:

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

Code: `src/contrast.rs` (`pick_stage`) and `effective_backdrop` / `is_dark` in `src/gerar.rs`;
tests in both.
