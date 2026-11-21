# Samples

The demo file for Handoff Components is the **DS Base Demo**, in [`../../demo/`](../../demo/README.md)
(`demo/ds-base-demo.figcraft`, a small neutral design system built from scratch; MIT).
The sample documentation images in this folder, `report-*.png`, are the ones the main README shows: `report-cover`, `report-master`, `report-properties`,
`report-anatomy`, `report-variants` and `report-layouts` are frames of a Section that the 0.11.0 plugin made for **a test component** ("Teste · Card", its content in
Portuguese), exported from the app as 2x PNG and reduced to 256 colors. `report-card.png` is the only one left of the older series: the Card of the DS Base Demo, made with
an earlier version of the plugin, so it does not show the 0.11.0 layout. The `relatorio-*.png` files are older, historical samples (orange lines, older layouts) and are not regenerated.

This folder also holds the recorded outputs of the probes and commands the plugin was built on, taken from
the test fixtures in `tests/fixtures/` (`fixture-copia`, `fixture-copia-2`, `fixture-carga-v2`;
see `LEIA-ME.md`, in Portuguese). They are kept as reference for the host's command formats.

## Generating the documentation

1. Build the plugin with `scripts/build.sh`. It prints the absolute path of the manifest to import.
2. In FigCraft: **Plugins ▸ Development ▸ Import plugin from manifest…**, pick `dist/figcraft-plugin.json`.
3. Open `demo/ds-base-demo.figcraft`, select a component or a set, run **Plugins ▸ Handoff Components ▸ Generate documentation…** and press **Generate** in the panel.
   A new Section goes to the right of everything on the page (256 px from it), at y = 256; a rerun replaces the frames Update names, in their place.

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

The six new ones were exported from the app: generate the documentation for the test component, select a frame of the Section and export it as PNG at 2x. The images are not made by a script.
