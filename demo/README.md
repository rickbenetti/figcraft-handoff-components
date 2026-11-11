# DS Base Demo

A small, neutral design system built from scratch to demo the Handoff Components plugin for FigCraft.
Clean room: it does not derive from any client file. Colour tokens follow shadcn/ui (MIT); names and
measures are its own. Licence: MIT (see `LICENSE`).

- `ds-base-demo.figcraft`: the file (pages **Components** and **Foundations**).
- `build_ds.py`: rebuilds it headless: `python3 build_ds.py` (or `python3 build_ds.py out.figcraft`; needs `figcraft-cli` on the PATH).
  `mcpc.py` is its tiny MCP client; `phosphor_regular.json` holds the ten icon paths.
- `THIRD_PARTY_NOTICES.md`: licences of the icons and fonts.
- `contrast_check.py` and `CONTRAST.md`: WCAG contrast of every text and icon pair in Light and Dark (the tool reads the file, the table is its output).
- `FROZEN.md`: the frozen state (SHA-256, sets, axes).
- `preview/`: PNG previews exported from the file (Button, Card, Accordion, Icon, Code, Dark preview, and the `-dark` artboards).

Fonts: Inter and JetBrains Mono (SIL OFL); Icons: Phosphor Icons (MIT), see THIRD_PARTY_NOTICES.md.

## What is in it

| | |
|---|---|
| Variables | **Colors** (15 base + 6 derived: hover and pressed for primary, destructive and secondary; modes Light / Dark), **Spacing** (4, 8, 12, 16, 24), **Radius** (10) |
| Styles | 8 text (Inter, plus JetBrains Mono for code), 2 effect (`shadow/sm`, `shadow/md`), 1 paint (`surface/code`) |
| Components | **Icon** (10 Phosphor icons × 3 sizes, 30 variants), **Button** (60 variants: Type × Size × State, icons as swappable instances, hidden trailing icon; hover and pressed have colours of their own), **Card** (Compact / Default / Wide: wrapper, footer actions, long text, 3 identical rows), **Accordion** (Collapsed / Expanded / Locked), **Code** (monospace, 3 identical lines) |

On the Components page every component has a twin artboard named `<Component> / Dark`: a frame bound to `Colors/background`
with its variable mode set to Dark, holding instances only (no loose colour, no override).

Edge cases on purpose: hidden layers (the trailing icon, the Card features at Compact, the Accordion answer when collapsed), text
styles next to loose type values (Button Small, Card Compact title), effects (hover shadow), values bound to tokens mixed
with loose ones (Button Small radius 8, gap 6, vertical padding 4), a plain colour kept as
a style (`surface/code`), 50% opacity on the disabled and locked states, and repeated identical layers (x3).

## Known limitations

- The focus ring (token `ring`) is about 2.5:1 against white in Light, below the 3:1 that WCAG 2.2 asks for focus
  indicators. It is the value of the base theme, kept as is.
