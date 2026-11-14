# Data contract (0.10.0)

What Handoff Components stores in the file, what it accepts as parameters, and what the generate command returns.
Every row names the file and function it comes from. Source files are in `src/`.

## 1. Plugin data the plugin writes

`pluginData` values are strings. Source: `store.rs` (module doc, `SETTINGS_KEY`, `REPORT_KEY`).

| Key | On | Value | Written by | Read by |
|---|---|---|---|---|
| `handoff-report` | the documentation frame | The id of the component set (when a set is documented) or of the component, as text. It marks the frame so a rerun finds it. The key keeps its old name on purpose. | `gerar.rs` `generate()` (the `frame.create` call, `"pluginData": {REPORT_KEY: key}`); `key` is `target.key` (`gerar.rs`, struct `Target`) | `generate()`: finds the last child of the page with this key equal to the key of the run (`gerar.rs`, the `old` lookup) |
| `handoff-settings` | the current page | A JSON object as text, with only the fields that are set: `lang`, `on_rerun`, `stage`, plus any other key it already held (section 2). An object with nothing in it is removed. | `store.rs` `Settings::to_text`; `direct.rs` `write_data`; the panel's `plan()` (`panel.rs`) | `store.rs` `Settings::from_plugin_data`. Absent, empty, broken text or a JSON that is not an object reads as no settings. |

Nothing else is remembered: **Version** and **Status** are typed for each documentation and are not saved
(`store.rs` module doc; `panel.rs` `Form::after_generate`).

## 2. The page's settings (`handoff-settings`)

| Field | Accepted values | Default | Notes | Source |
|---|---|---|---|---|
| `lang` | `en`, `pt-BR` (`pt` is read as `pt-BR`) | `en` | `pt` is rewritten as `pt-BR` the next time the settings are saved. A value that names no language is kept as it is. | `direct.rs` `canonical_lang`; `i18n.rs` `Lang::strict`/`Lang::parse` |
| `on_rerun` | `replace`, `new` (any case) | `replace` | `replace`: delete the old documentation of this component and draw the new one where it was. `new`: keep the old one and draw the new one to the right of everything. | `store.rs` `Rerun::parse`, `Rerun::default` |
| `stage` | `light`, `dark`, `auto` (any case) | `auto` | The stage behind the instances. `auto` chooses row by row from contrast. | `store.rs` `StageMode::parse`, `StageMode::default` |

**Ignored but kept.** Any other key in the stored object goes into `Settings.other`: it is not used and is written back as it
was when the settings are saved (`store.rs` `Settings::from_plugin_data`, `to_text`; tests `a_stored_time_zone_is_ignored_but_kept`,
`a_stored_line_colour_or_opacity_is_ignored_but_kept`). Keys earlier versions wrote:

| Key | Was (earlier versions) | Now | Source |
|---|---|---|---|
| `tz` | time zone of the generation date | ignored, kept | `store.rs` module doc |
| `line` | colour of the guide lines | ignored, kept; the colours are fixed per stage and section | `direct.rs` module doc; `store.rs` `Settings` doc |
| `fill_opacity` | opacity of the measure highlight | ignored, kept | same |
| `fill` | colour of the measure highlight | ignored, kept | `panel.rs` `plan()` comment; test `a_field_the_user_did_not_change_keeps_its_stored_text_exactly` |

**Restore defaults** (panel): clears `on_rerun` and `stage`, keeps `lang`, and when all restorable fields are cleared it empties
`other` too, so the leftover keys above are removed (`panel.rs` `Form::restore_page_defaults`, `plan()`; test
`restore_defaults_keeps_the_language_and_removes_the_other_keys_that_are_stored`). Gap in the docs: the README names only `tz`, `line` and
`fill_opacity` as removed by Restore defaults; by the code `fill` goes too.

### Parameters of the direct run (no panel)

A script runs the command with `direct: true` (any other value, or no `direct`, opens the panel). Source: `direct.rs` `wanted`, `run`, `page_from_params`.

| Parameter | Accepted | Effect |
|---|---|---|
| `direct` | `true` (boolean) | Skip the panel. |
| `lang`, `stage`, `on_rerun` | the values of section 2; empty keeps what is stored; `-` clears it; anything else is an error with the dictionary's message | Written to the page's settings (only if they changed). |
| `version`, `status` | text, control characters become spaces, trimmed, at most 10,000 characters | The documentation's header only; nothing is saved (`gerar.rs` `meta_text`, `MAX_META_TEXT`). |
| `line`, `fill_opacity` | anything | Ignored (`direct.rs` module doc; test `a_bad_value_is_refused…`). |

## 3. What generate returns (JSON)

Source: the `Ok(json!({...}))` at the end of `generate()` in `gerar.rs` (the settings and direct paths call it: `direct.rs` `run`).
This is what a **direct run** (`direct: true`) returns. A click on **Generate documentation** in the panel returns a smaller object (section 3.6). Types are the ones the code builds; "id" is a FigCraft layer id (number).

### 3.1 Top-level fields

| Field | Type | Meaning |
|---|---|---|
| `name` | string | The set's name for a set, else the component's (`Target.title`). |
| `frame` | id | The documentation frame. |
| `stage` | id | The anatomy stage. |
| `anatomy_row` | id | The row that holds the anatomy stage and the legend. |
| `legend` | id | The legend frame. |
| `instance` | id | The instance placed on the anatomy stage. |
| `component` | id | The component that was documented (for a set, its default variant). |
| `rects` | array of id | Measure outlines drawn on the spacing stage (`draw_overlay`). |
| `labels` | array of id | Measure labels drawn on the spacing stage. |
| `markers` | array of id | Numbered markers on the anatomy stage (`Badges.ids`). |
| `x`, `y` | number | Where the frame was put: where the replaced one was, else 512 px right of everything (`GAP_RIGHT`) at `y = 0`. |
| `replaced` | id or null | The id of the old documentation that was deleted (only with `on_rerun = replace`). |
| `rerun` | `"replace"` or `"new"` | The mode used. |
| `top_level_layers` | number | How many layers the page had before the run's delete. |
| `id_source` | string | Where the selection id came from (a fixed text: the run's `selection`, not a FigCraft command). |
| `settings` | object | See 3.2. |
| `meta` | object | `{version: string, status: string}`: the header's text (empty is shown as "to be defined"). |
| `notes` | array of string | Messages for the user (what could not be drawn or was left out). |
| `lang` | `"en"` or `"pt-BR"` | Language of the documentation (same as `settings.lang`). |
| `selected` | string | `"ok"`, or the host's error when selecting the frame was refused (does not fail the run). |
| `instance_size` | `{w, h}` | Size of the instance on the anatomy stage. |
| `stage_size` | `{w, h}` | Size of the anatomy stage. |
| `frame_size` | `{w}` | Width of the frame. |
| `legend_height` | number | Height of the legend. |
| `audited_layers` | number | Layers read from the component. |
| `rows` | object | `{all_properties, in_table, lines_after_grouping, hidden}`, all numbers: property rows found, rows kept in the table, lines after grouping layers, lines hidden by the limit (`MAX_AUDIT_LINES` = 250). |
| `legend_cost` | object | `{lines, blocks, host_calls}`, numbers: what the legend cost. |
| `spacing` | object or null | `{stage: id, instance: id, stage_size: {w, h}, legend_height}` for the spacing stage; null when the component has no spacing section. |
| `spacing_wrapper` | number or null | Index of the wrapper layer measured instead of the root (a component whose only wrapper has the spacing); null otherwise. |
| `export` | object | See 3.3. |
| `badges` | array | `{n: number, layer: string or null}`: the number of each marker and its layer name. |
| `properties` | array | `{name, type, options: [{option, default}]}` for each component property, with duplicates numbered ("Message 1"). |
| `default_variant` | object or null | `{pairs: string, source: "Component" or "First"}`; null when a component, not a set, was documented. |
| `axes` | object or null | See 3.5; null when there are no variant axes. |
| `effects` | object | See 3.4. |
| `stage_mode` | `"light"`, `"dark"` or `"auto"` | The stage setting used. |
| `anatomy_stage` | object | `{dark: boolean, lowest_contrast: number or null}`: whether the anatomy stage is dark and the lowest contrast found. |
| `host_calls` | number | Host calls the run made. |
| `badge_info` | array | `{n, centre: [x, y], end: [x, y]}` for each marker. |
| `badges_dropped` | array of number | Numbers with no marker on the stage (no clear room). |
| `unnumbered_layers` | number | Layers left without a number (`number_blocks`). |
| `anatomy_blocks` | array | `{title: string, badge, count: number}` per block of the legend. |
| `hidden_paints` | number | Hidden fills and strokes found in the audit. |
| `summary` | string | The header's summary line. |
| `warning` | string or null | The collision warning; null when there is none. |
| `table` | array | One object per line of the properties table: `{layer, type, layers, property, token, value, state, count}` (`property` is always the English name; `state` is the Rust debug name of the row state). |

### 3.2 `settings`

| Field | Type | Meaning |
|---|---|---|
| `lang` | string | Language code used. |
| `palette` | `{anatomy: string, spacing: string}` | The fixed colours (hex) of the anatomy and the spacing sections for the stage that was chosen (`gerar.rs` `palette`, `theme.rs`). |
| `fill` | string or null | The colour of the measure highlight (the spacing colour at 30%, `#RRGGBBAA`), for information only (`RedStyle.overlay_fill`); it is not a setting. |

### 3.3 `export`

| Field | Type | Meaning |
|---|---|---|
| `format` | `"svg"` | Always SVG. |
| `size` | `"1x"` | Always `1x` (the scale is ignored for SVG). |
| `result` | string | `"ok"` or the host's error text when adding the export setting failed. |
| `frame_height` | number | Height of the frame. |

### 3.4 `effects`

| Field | Type | Meaning |
|---|---|---|
| `reach` | number | How far the instance's effects (shadows, blurs) reach out of it, in px; 0 when none; at most `cap`. |
| `cap` | number | 400 (`EFFECT_CAP`): the most a stage grows per side. |
| `slack` | number | 8 (`EFFECT_SLACK`): room kept beyond the reach. |
| `narrow` | array | `{name, needed_width, stage_width}` for each stage narrower than the instance plus the reach on both sides. |

### 3.5 `axes` (null when there are no variant axes)

| Field | Type | Meaning |
|---|---|---|
| `rows` | number | Axis rows drawn. |
| `skipped` | number | Rows skipped. |
| `undefined` | array of string | Combinations that do not exist as variants. |
| `planned` | number | Rows planned within the limit (`MAX_AXIS_ROWS` = 24). |
| `stages` | array | One per row: `{name, lowest_contrast, dark, height, effect_reach, deltas: [string]}` (`deltas` in English). |
| `stacked` | object | `{limit, used, capped, measured, redrawn}`, numbers: `limit` = 400 nodes (`stacked.rs` `BUDGET`); `used` = nodes spent; `capped` = changes left as one line because the budget ran out; `measured`, `redrawn` = grids drawn to be measured and drawn again. |

### 3.6 What the panel returns after **Generate documentation**

Source: `panel.rs` `generate_click`, `show_start`. The panel calls `generate()` with only `version` and `status`, then moves the view and puts the selection back.

| Field | Type | Meaning |
|---|---|---|
| `phase` | `"generate"` | Which callback ran. |
| `refused` | boolean | `true` when nothing was generated (no selection, or a selection that is not a component, a component set or an instance). |
| `message` | string | The message the panel shows, in the page's language. |
| `frame` | id | The new documentation frame (only when `refused` is `false`). |
| `view` | object | The view after moving to the start of the frame: `{zoom, cx, cy, vw, vh, asked: {zoom, cx, cy}}`, or `{error: string}` when it did not move (not fatal). |
| `selection` | array of id | The selection restored after the run. |
| `meta` | object | `{version, status}` as in 3.1. |

The results of the other panel callbacks (Restore defaults, Close, field edits) are not described here.

## 4. Differences from 0.9.6 (field by field)

Compared with `git show v0.9.6:src/gerar.rs` (the `Ok(json!({...}))` of `generate()`) and `v0.9.6:src/store.rs`.

| Where | 0.9.6 | 0.10.0 |
|---|---|---|
| `name` (top level) | absent | **added** |
| `generated` (`date`, `created`, `tz`, `failure`, `leftover_comment`) | present | **removed** (no date) |
| `settings.line` | the line colour | **removed** |
| `settings.fill` | an adjustable fill colour | kept, but **only informative** (the measure highlight) |
| `settings.palette` | absent | **added** `{anatomy, spacing}` |
| `export.format` | absent | **added**, always `"svg"` |
| `export.size` | `"1x"` or `"2x"` (by height) | always `"1x"` |
| `effects` (top level) | absent | **added** |
| `axes.stacked` | absent | **added** |
| `meta` | `{version, status}` | same |
| Page `handoff-settings`: `line`, `fill`, `tz` | settings fields (`Settings{lang, line, fill, on_rerun, stage, tz}`) | **ignored and kept** in `other` |
| Page `handoff-settings`: `fill_opacity` | not in 0.9.6 (it existed only in commits between the releases) | ignored and kept |
| Page `handoff-settings`: `lang`, `on_rerun`, `stage` | same fields | same fields |
| Frame `handoff-report` | the component's or set's id | same |
| Manifest | `api` 1, permissions include `comment.list`, `comment.add`, `comment.delete`, a `settings` command | `api` 2, permissions include `view.get`, `view.set`; no `settings` command |

Other top-level fields (`frame`, `stage`, `anatomy_row`, `legend`, `instance`, `component`, `rects`, `labels`, `markers`, `x`, `y`,
`replaced`, `rerun`, `top_level_layers`, `id_source`, `notes`, `lang`, `selected`, `instance_size`, `stage_size`, `frame_size`,
`legend_height`, `audited_layers`, `rows`, `legend_cost`, `spacing`, `spacing_wrapper`, `badges`, `properties`, `default_variant`,
`stage_mode`, `anatomy_stage`, `host_calls`, `badge_info`, `badges_dropped`, `unnumbered_layers`, `anatomy_blocks`, `hidden_paints`,
`summary`, `warning`, `table`) are present in both versions; this document does not claim that their values are unchanged, only that the keys are.

## 5. `handoff-meta`

Earlier versions saved a `handoff-meta` sheet on each component (version, status, default variant, and a typed `date` before 0.9.6).
The plugin **does not read it and does not delete it**; it stays in the file as it was (`store.rs` module doc; `gerar.rs` `generate()`: "a sheet
an older version saved on the component is not read").

## Gaps

- The meaning of `rects`, `labels`, `markers`, `badge_info` and `legend_cost` is read from the code's structure; the code does not document them.
- `table[].state` is the Rust debug name of an internal enum; its set of values is not listed here.
- `axes.stacked.measured` and `redrawn` are named from the code; their exact unit (grids or nodes) is not documented in the source.
