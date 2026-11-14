# Samples

The demo file for Handoff Components is the **DS Base Demo**, in [`../../demo/`](../../demo/README.md)
(`demo/ds-base-demo.figcraft`, a small neutral design system built from scratch; MIT).
Sample documentation images made from it (Button, Card, Accordion, Code) are in this folder: `report-*.png`. They are 2x PNG exports of the documentation frame (the DS Base Demo, `lang` en, version and status left at their defaults), made with the plugin at f0b6c49 (the fixed palette, the variant rows as blocks with only the option's value as title, the stacked format of the differences in striped blocks); the Card and Code ones come out on the dark stage because the stage is Auto. They are rendered by `figcraft-cli` (no app window), with the fonts of the machine. The Code image has no variant rows, so it has not changed since 953bb57. The `relatorio-*.png` files are older, historical samples (orange lines, older layouts) and are not regenerated.

This folder also holds the recorded outputs of the probes and commands the plugin was built on, taken from
the test fixtures in `tests/fixtures/` (`fixture-copia`, `fixture-copia-2`, `fixture-carga-v2`;
see `LEIA-ME.md`, in Portuguese). They are kept as reference for the host's command formats.

## Generating the documentation

1. Build the plugin with `scripts/build.sh`. It prints the absolute path of the manifest to import.
2. In FigCraft: **Plugins ▸ Development ▸ Import plugin from manifest…**, pick `dist/figcraft-plugin.json`.
3. Open `demo/ds-base-demo.figcraft`, select a component or a set, run **Plugins ▸ Handoff Components ▸ Generate documentation…** and press **Generate documentation** in the panel.
   A new documentation goes 512 px to the right of the rightmost edge of everything on the page, with its top at y = 0; a rerun that replaces an earlier one takes its place.

From a terminal (always write to another file, never over the original; `direct` makes it with no panel):

```sh
figcraft-cli run --in demo/ds-base-demo.figcraft --out /tmp/demo-out.figcraft script.json
```

where `script.json` imports the plugin, selects a layer (look the id up by name first, e.g. with `node.find`) and runs it:

```json
[{"command": "plugin.import", "params": {"path": "dist"}},
 {"command": "select.set", "params": {"ids": [623]}},
 {"command": "plugin.run", "params": {"id": "me.benetti.handoff-components", "command": "generate", "params": {"direct": true}}}]
```

### How the `report-*.png` images are made

Run the plugin on the component from `demo/ds-base-demo.figcraft` (Button, Card, Accordion: the sets; Code: the component) with `direct` and `lang: en`, save to another file, then export the documentation frame at 2x:

```sh
figcraft-cli export out.figcraft --node <frame id> --format png --scale 2 --out report-card.png
```

The frame id is `output.frame` in the `plugin.run` result. A PNG of a frame taller than 8,192 px at 2x would pass 16,384 px, so a taller one needs 1x.
