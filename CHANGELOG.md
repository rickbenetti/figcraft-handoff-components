# Changelog

All notable changes to Handoff Components. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Version 0.x: expect rough edges. The data the plugin reads and writes is described in
[`docs/data-contract.md`](docs/data-contract.md).

## [0.10.0] - 2026-11-14

### Requirements

- A FigCraft that runs **plugin API 2** (the manifest says `"api": 2`) and offers the commands `view.get` and `view.set`.
  The plugin is tested with **figcraft-cli 0.18.1**, built against `figcraft-plugin-sdk` tag `v0.14.0`.
- **Minimum FigCraft version: not verified.** What the sources say: FigCraft refuses a manifest that asks for a newer
  plugin API than it runs ("plugin API 2 isn't supported (this FigCraft runs API N)"), so on an older FigCraft the plugin
  fails at import, not halfway through a run (FigCraft's `docs/plugins.md`, the `api` row, and `crates/plugin/src/manifest.rs`).
  Plugin API 2 first shipped in the FigCraft tag `v0.4.0` (2026-10-12; the commit "M7: plugin panels and settings" is in
  `v0.4.0` and not in `v0.3.1`), and `view.get`/`view.set` are in the engine since `v0.2.0`. That makes `v0.4.0` a lower
  bound read from the source history, not a version this plugin was run on. Whether a FigCraft older than the SDK's
  `v0.14.0` runs this plugin's panel and commands is not verified.

### Added

- **A panel for "Generate documentation" (plugin API 2).** Running the command opens a panel that stays open until you
  close it. It has 17 controls (notes, fields and buttons): language, stage (light, dark or auto), what to do when the component already has a
  documentation (replace or new), and the **Version** and **Status** typed for each documentation; **Generate
  documentation**, **Restore defaults** and **Close** buttons come after the fields. The layout is the same in every state
  and in both languages.
- After generating, the screen moves to the new documentation.
- **Variant differences in blocks.** Each variant axis row is a block whose title is only the option's value.
  A change is drawn stacked: a label, a pill with the token and a colour swatch with its value; the blocks are striped,
  with bold labels.
- **Instance swaps** are described in the variant differences, and the **default component of a swap property** shows in
  the properties table.
- **Properties that share a name are numbered** ("Message 1", "Message 2"), and an empty text default is said.
- Identical changes are **grouped under one title** that lists every layer name.
- A long text change is cut at a word boundary with a `[…]` marker.
- The stage keeps **room for the instance's effects** (shadows, blurs): up to 400 px on each side.
- The generated JSON has new fields: `settings.palette`, `export`, `effects`, `axes.stacked`
  (see the data contract).

### Changed

- **Plugin API 2.** The manifest's permissions are `view.get` and `view.set` instead of `comment.list`, `comment.add`
  and `comment.delete`.
- "Report" is now **"documentation"** in every text the user sees. The documentation has no generation date.
- The header shows **Version** and **Status** always; its summary puts the total first and calls it "design values".
- Colours are **fixed per stage and per section**; the guide and measure outlines are drawn at 1.5 px.
- The documentation is placed **512 px to the right of everything on the page, at y = 0**.
- **Export is always SVG** (1x, no suffix), whatever the height.
- Width and height of a component that hugs or fills are no longer listed as differences.
- Colour alpha is shown in **whole percent**; layer opacity keeps **one decimal**.
- The stacked format of the changes spends at most **400 nodes** per run; what does not fit is left as one line.
- **pt-BR:** terms revised (opacity label, the "replace or new" options, numbers, the warning, "clique em"); property and
  difference lines use `": "` as the separator, in both languages.
- README, `docs/` and the sample images show the new documentation.

### Removed

- The **Settings** command and its five fields (replaced by the panel).
- The **time zone** setting and the **generation date**.
- The **purple outline** of the instance on the stage.
- The **8,000 px rule** and **PNG** export (only SVG now).
- The adjustable **fill colour** and **line colour** of the guides and measures: `line`, `fill` and `fill_opacity`
  are no longer settings. A value an earlier version stored is ignored and kept as it was; **Restore defaults** drops it.
- The permissions `comment.list`, `comment.add` and `comment.delete`.

### Fixed

- The placement text of the documentation (where it is put) was wrong; fixed, and the README sample images were regenerated.
- Large shadows and blurs are no longer clipped by the stage.

### Known limitations

- **Not seen in the app with a window.** Everything above that depends on how the documentation looks (images, fuel,
  SVG) was checked with `figcraft-cli` (headless). The panel was exercised through the CLI's panel sequence
  (`scripts/panel_sequence.py`), not by clicking in the app.
- The hex values of the colour swatches use the font Inter Light; without it installed, they fall back to Regular
  (not verified in the app).
- Not tested in a shared (collaborative) session or in a file opened for viewing only.
- The screen freezes while the documentation is generated.
- See the README for the rest.

## [0.9.6] - 2026-11-11

See the [0.9.6 release](https://github.com/rickbenetti/figcraft-handoff-components/releases/tag/v0.9.6).
