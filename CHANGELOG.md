# Changelog

All notable changes to Handoff Components. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Version 0.x: expect rough edges. The data the plugin reads and writes is described in
[`docs/data-contract.md`](docs/data-contract.md).

## [0.11.0] - unreleased

### Requirements

- Same as 0.10.0: FigCraft **plugin API 2**, tested with **figcraft-cli 0.18.1**. The plugin now asks for seven more
  permissions (see Changed).

### Added

- **The documentation is a Section.** Each component gets one Section on the page with **eight frames in one column**:
  Cover, Component Master, Properties, Anatomy, States and Variants, Layouts, Tokens and Changelog (1180 px wide, 64 px
  apart). Heights are read back from what the host laid out, so the gap between frames is 64 px.
- **Cover.** Name, version and status guides you type over, last designer, date, description, and six **approval pills**
  (Designer, Engineering, QA, Writers, PO, PM) with a legend of the four states. It is made once; a rerun never changes it.
- **Support frame.** One frame for the whole file holds the **set of approval pills** (`.Handoff · Approval`) the Covers
  use. It stands to the left of the first Section. The pills are written with the property names of the set that is in the file,
  whatever language made it. Do not delete it; if you do, a Cover that is made again makes it again.
- **Component Master.** The original component or set moves into this frame, so the instances elsewhere in your file stay
  linked to it. A set wider than the frame is arranged with **auto layout and wrap**, only when it does not fit; a set that fits is
  left as you made it. The Master is made again on every run.
- **Properties** and **Tokens** are frames of their own: Properties (name, type, default, options), and Tokens (tokens by
  layer, and resolved colors in Dark and Light).
- **Anatomy** shows the instance with numbered markers on straight guides only (horizontal or vertical). Guides never
  cross a text, markers keep their distance from each other, and a number that has no room is said in the table's Note column.
- **States and Variants** is a gallery: one tile with a real instance for each option of each variant axis, with its
  name and a line that says `default` or `changes: Fill, Border, …` (names only, never values). The default option is always
  there; a budget limits how many tiles are drawn and the rest is counted.
- **Layouts** draws the padding and the gap as **translucent bands** over the instance (padding green, gap violet), **one
  drawing for each level of nesting** (up to 4), with the value as a number. Equal bands share one number tied to them by a
  vertical line and short traces, on each side of the instance; a number never covers text, vectors or icons.
- **Update.** The panel's new **Update** choice (all sections, tables only, tokens only, properties only, anatomy only,
  states and variants only, layouts only) says which frames a rerun replaces. The first run always makes all eight.
- **New Section** makes another Section for the same component beside the first (without a Master).
- **Selecting a variant** documents the whole set, as if you had selected the set.
- **A frame you deleted by hand** comes back, in its place, when Update names it. When Update does not name it, the panel
  says it was not made.
- **Warnings.** A run can now tell you, with a short message in the panel: `table_failed` (a frame could not be made),
  `no_stage` (no stage suits every variant), `reflow_failed` (the set could not be arranged in lines),
  `wide_master` (a very wide component: every frame was made wider), `legacy_doc` (a 0.10.0 documentation was found and left as it
  was), `approval_property_missing` (the approval pills have no Area property), `support_missing` (the support frame could not be
  made again), `frame_missing` (a frame is missing and Update does not name it), `markers_dropped` (markers that did not fit the
  Anatomy) and `spacing_dropped` (spacing bands that did not fit the Layouts drawing).
- The answer of a run has new fields (`section`, `frames`, `master`, `anatomy`, `gallery`, `spacing`, `tokens`, `support`,
  `warnings`, `missing`, `migrated_from`, …); see the data contract.

### Changed

- **One Section instead of one frame.** The documentation is no longer one frame placed 512 px to the right of everything:
  a Section goes to the right of everything on the page (256 px) and holds the frames above.
- **A shorter panel** (11 controls): language, stage, Update, and the buttons **Generate**, **New Section**, **Restore defaults** and
  **Close**. The **Version** and **Status** fields and the **On rerun** setting are gone; you type version and status on the Cover.
  **Restore defaults** puts Language, Stage and Update in the panel back to their defaults; Language and Stage are then saved with the next documentation, and Update is never saved (it is All each time the panel opens).
- **Anatomy** is the stage plus a table of the numbers (No., Type, Layer, Note) instead of the legend.
- **The stage** can now also be **mid-grey** (`#777777`): Auto chooses among light, mid-grey and dark, and for the Component Master it
  judges every variant of the set (a sample in a big set), where 0.10.0 judged each picture alone. The Stage control still offers
  Auto, Light and Dark; mid-grey comes only from Auto.
- The panel shows **one** warning at a time; one that says something was not made comes before advice.
- **Permissions.** The manifest asks for seven more host commands: `node.create`, `component.create`,
  `component.combineVariants`, `instance.create`, `component.addProperty`, `component.bindProperty` and `instance.setProperties`
  (the Section, and the approval pills as a real component).

### Removed

- The **variant matrix** (one row of blocks for each axis) and the stacked format of the changes: States and Variants is a gallery now.
- The **legend of each numbered layer's properties and tokens** beside the Anatomy (the tokens and values of each layer are in
  the Tokens frame now), and the 0.10.0 measure rectangles of the spacing (`measure.padding`).
- The header with **Version** and **Status** and the **On rerun** control. A script can still pass `on_rerun`, `version` and `status`
  to a direct run; they are accepted and **ignored**.

### Known limitations

- **Translucent bands over colored components.** The Layouts bands are translucent, so over a colored component (a blue
  button, say) they change its color; the numbers stay readable, the colors do not.
- The **Layouts frame is tall** when the instance is tall (one drawing for each level of nesting).
- **Small instances stay small.** The host cannot scale an instance whole, so the drawings are at 1x.
- The panel shows **one warning at a time**, even when a run has several.
- `on_rerun`, `version` and `status` are **accepted and ignored** in a direct run.
- Spacing bands that do not fit the drawing are not drawn; the note under the drawing says how many.
- **How it was tested.** The measurements and the fuel in this repository come from `figcraft-cli 0.18.1` without a window
  (synthetic).

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
  are no longer settings. A value an earlier version stored is ignored and kept as it was; **Restore defaults** drops it when the next documentation is generated.
- The permissions `comment.list`, `comment.add` and `comment.delete`.

### Fixed

- The placement text of the documentation (where it is put) was wrong; fixed, and the README sample images were regenerated.
- Large shadows and blurs are no longer clipped by the stage.

### Known limitations

- **How it was tested.** The sample images, the measurements and the fuel in this repository come from
  `figcraft-cli 0.18.1` without a window (synthetic). The panel and the generated documentation were checked by hand in
  FigCraft 0.18.1.
- **Not tested:** other FigCraft versions; a shared (collaborative) session; a file opened for viewing only; the font
  Inter Light of the hex values without the font installed (it falls back to Regular).
- The screen freezes while the documentation is generated.
- See the README for the rest.

## [0.9.6] - 2026-11-11

See the [0.9.6 release](https://github.com/rickbenetti/figcraft-handoff-components/releases/tag/v0.9.6).
