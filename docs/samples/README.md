# Samples

The demo file for Handoff Components is the **DS Base Demo**, in [`../../demo/`](../../demo/README.md)
(`demo/ds-base-demo.figcraft`, a small neutral design system built from scratch; MIT).
Sample documentation images made from it (Button, Card, Accordion, Code) are in this folder: `report-*.png`. These images were made before the fixed palette, the instance effects margin and the `Label: value` separator (the lines are the old orange); they have not been regenerated yet.

This folder also holds the recorded outputs of the probes and commands the plugin was built on, taken from
the test fixtures in `tests/fixtures/` (`fixture-copia`, `fixture-copia-2`, `fixture-carga-v2`;
see `LEIA-ME.md`, in Portuguese). They are kept as reference for the host's command formats.

## Generating the documentation

1. Build the plugin with `scripts/build.sh`. It prints the absolute path of the manifest to import.
2. In FigCraft: **Plugins ▸ Development ▸ Import plugin from manifest…**, pick `dist/figcraft-plugin.json`.
3. Open `demo/ds-base-demo.figcraft`, select a component or a set, run **Plugins ▸ Handoff Components ▸ Generate documentation…** and press **Generate documentation** in the panel.
   The documentation goes to the right of everything on the page, at the same height.

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
