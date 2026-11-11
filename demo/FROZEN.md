# DS Base Demo: frozen state

- Date: 2026-11-10
- File: `ds-base-demo.figcraft` (106,645 bytes)
- SHA-256: `55ecb92652984e791bb11e06c8fca128f9c783ec3985637a59fff11bcfb8cc57`
- Plugin: Handoff Components checks this file in `scripts/regress.sh` (the DS Base Demo cases). No plugin version or build hash is recorded here: they change with every build, and the file above is what stays frozen.
- From here on the file changes only on request. Rebuild with `python3 build_ds.py`; the regress will find components by NAME (ids below are information only and change if the build script changes).

## Variables and styles

- **Colors** (Light, Dark): 21 variables: color/background, color/foreground, color/card, color/primary, color/primary-foreground, color/secondary, color/secondary-foreground, color/muted, color/muted-foreground, color/accent, color/accent-foreground, color/destructive, color/border, color/input, color/ring, color/primary-hover, color/primary-pressed, color/destructive-hover, color/destructive-pressed, color/secondary-hover, color/secondary-pressed
- **Spacing** (Mode 1): 5 variables: space/4, space/8, space/12, space/16, space/24
- **Radius** (Mode 1): 1 variables: radius/default
- **Styles** (11): `text/h3` (text), `text/body-large` (text), `text/body` (text), `text/muted` (text), `text/label-s` (text), `text/label-m` (text), `text/label-l` (text), `text/code` (text), `shadow/sm` (effect), `shadow/md` (effect), `surface/code` (paint)

## Pages, components and sets (by name)

| page | name | kind | axes and values | id |
|---|---|---|---|---|
| Components | Icon | component set (30 variants) | **Name**: caret-down, caret-up, caret-right, plus, minus, x, check, arrow-right, info, warning (default caret-down); **Size**: 16, 20, 24 (default 16) | 208 |
| Components | Button | component set (60 variants) | **Type**: Primary, Secondary, Outline, Destructive (default Primary); **Size**: Small, Medium, Large (default Medium); **State**: Enabled, Hover, Pressed, Disabled, Focus (default Enabled); Leading icon (boolean, default True); Trailing icon (boolean, default False); Leading glyph (instanceSwap); Trailing glyph (instanceSwap) | 629 |
| Components | Card | component set (3 variants) | **Size**: Compact, Default, Wide (default Default) | 741 |
| Components | Accordion | component set (3 variants) | **State**: Collapsed, Expanded, Locked (default Expanded) | 763 |
| Components | Code | component | none | 764 |
| Components | Button / Dark | frame (Dark mode, instances only) | | 768 |
| Components | Card / Dark | frame (Dark mode, instances only) | | 825 |
| Components | Accordion / Dark | frame (Dark mode, instances only) | | 863 |
| Components | Code / Dark | frame (Dark mode, instances only) | | 885 |
| Components | Icon / Dark | frame (Dark mode, instances only) | | 890 |
| Foundations | Type styles | frame | | 3 |
| Foundations | Effect specimens | frame | | 12 |
| Foundations | Colors / Light | frame | | 16 |
| Foundations | Colors / Dark | frame (Dark mode, swatches) | | 82 |
| Foundations | Dark preview | frame (Dark mode, instances only) | | 907 |

## Contrast

`CONTRAST.md`: 167 text and icon pairs in Light and Dark (Disabled and Locked exempt), 0 below 4.5:1 (text) or 3:1 (icons). Tightest: muted-foreground on card / background in Light, 4.61:1.

## Previews (`preview/`)

button-set, card-set, accordion-set, icon-set, code, dark-preview, button-dark, card-dark, accordion-dark, code-dark, icon-dark (PNG).

## Notes

- The default variant of a set is the one at its top-left, so the Card row reads Default, Compact, Wide (Default stays the default; reordering would move the default).
- Disabled (Button) and Locked (Accordion) use 50% opacity on the whole component: the alpha case on purpose.
- Loose values on purpose (no token): Button Small radius 8, gap 6, vertical padding 4, and its label type values.
