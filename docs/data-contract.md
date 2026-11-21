# Data contract (0.11.0)

What Handoff Components stores in the file, what it accepts as parameters, what it draws, and what the generate command returns.
Every row names the file and function it comes from. Source files are in `src/`; `scripts/regress.py` is the check that holds the
numbers of sections 12 and 13. This document describes the code of the branch `feat/0.11-section-base`. The manifest
and `Cargo.toml` still say `0.10.0` (the version is not raised until the release; the manifest's permissions did not change: every host
command the code calls is in `figcraft-plugin.json`, and every command in it is called).

Numbers marked **synthetic** come from `figcraft-cli 0.18.1` without a window; the app may differ (section 14).

## Changes in 0.11.0 (since 0.10.0)

The CHANGELOG can rest on this list. Everything else in the document is the new state; section 16 keeps the 0.9.6 → 0.10.0 table.

| # | 0.10.0 | 0.11.0 | Where |
|---|---|---|---|
| 1 | One documentation frame on the page, marked `handoff-report` | **One Section** per component with **eight frames in one column** (Cover, Component Master, Properties, Anatomy, States and Variants, Layouts, Tokens, Changelog) | 3 |
| 2 | `handoff-report` marks the one frame | `handoff-report` marks **each of the eight frames** (same value); the Section has `handoff-report-section`; each frame has `handoff-section` (its kind) | 1 |
| 3 | `handoff-settings` on the page | Same key and fields; `on_rerun` is **kept but no longer used**; "Restore defaults" restores `lang` and `stage` (it used to keep `lang`) | 1, 2 |
| 4 | Every run replaced the frame (`on_rerun`) | A rerun replaces only the frames the **Update** choice names; **Cover and Changelog are made once**; the **Component Master is made again on every run**; new parameters `update` and `new_section` | 3.4, 11 |
| 5 | No Support frame | **One Support frame** for the whole document with the shared set of approval pills `.Handoff · Approval`, marks `handoff-support` and `handoff-approval-set` | 4 |
| 6 | The component stayed where it was | The original component or set is **moved into the Component Master**; a set wider than the room is given auto layout with **wrap** (mark `handoff-organized`) | 5 |
| 7 | One anatomy image with a legend | Anatomy stage + a table of numbered layers; only straight guides; `markers_dropped` | 7 |
| 8 | A matrix of rows per variant axis (`axes`, `stacked`) | A **gallery of instances**, one tile per option, with a node budget | 8 |
| 9 | Spacing drawn with `measure.padding` | **Bands** of padding and gap over the instance, **one drawing for each level** of nesting, **spines** for equal bands; `spacing_dropped` | 9 |
| 10 | The automatic stage chose between light and dark | Three candidates: light, mid-grey and dark (Anatomy, Layouts, Master, gallery tiles) | 5.2 |
| 11 | The generate JSON had `stage`, `legend`, `rects`, `labels`, `table`, `axes`, … | A different object (section 12): `section`, `frames`, `master`, `anatomy`, `gallery`, `spacing`, `tokens`, `tables`, `support`, `warnings`, … | 12 |
| 12 | `meta` (version, status) in the header; `version` and `status` parameters | **No header; the parameters are accepted and ignored** (the Cover has guides the person types over) | 11 |
| 13 | Warnings were free text in `notes` and `warning` | A list of **codes** in `warnings` (ten, section 10) | 10 |
| 14 | The panel returned `selection` and `meta` | The panel returns `refused`, `message`, `frame`, `view`, `selected`, `new_section`, `update`, `frames`, `warnings`, `stage` | 12.9 |
| 15 | Tokens in the legend of the anatomy | A **Tokens** frame with two tables (by layer; resolved colours in Dark and Light) | 6.2 |
| 16 | `handoff-meta` is not read | Same (never read, never deleted) | 16 |

## 1. Plugin data the plugin writes

`pluginData` values are strings. A **key** (`comp_key`) is the id of the component set when a set is documented (or when a variant of a set
is selected: section 3.6), else the id of the component, as text (`gerar.rs` `resolve`, `Picked.key`).

| Key | On | Value | Written by | Read by |
|---|---|---|---|---|
| `handoff-settings` | the current page | A JSON object as text with only the fields that are set (`lang`, `on_rerun`, `stage`) plus any other key it already held. An object with nothing in it is removed. | `store.rs` `Settings::to_text`; `direct.rs` `write_data`; `panel.rs` `plan()` | `store.rs` `Settings::from_plugin_data`. Absent, empty, broken or not-an-object text reads as no settings. |
| `handoff-report-section` | the **Section** | `comp_key` | `section.rs` `generate()` (`node.create`, `SECTION_MARK`) | `sections_of`, `find_section` (the page's own children), `find_support` |
| `handoff-report` | **each of the eight frames** of the Section (and, in files made by 0.10.0, the one documentation frame, on the page) | `comp_key` | `section.rs` `new_frame` (`REPORT_KEY`) | `has_legacy_frame`: a **page-level** frame with this key and no Section on a first run raises `legacy_doc` and is left alone |
| `handoff-section` | each frame of the Section | the frame's kind: `cover`, `master`, `properties`, `anatomy`, `variants`, `layouts`, `tokens`, `changelog` (and `kit`, see below) | `new_frame` (`NAME_MARK`, `Sec::key`) | `read_frames` (the first frame of each kind is the one used) |
| `handoff-support` | the **Support frame** (a top-level frame of the page) | `"1"` | `section.rs` `build_support` (`SUPPORT_MARK`) | `find_support` |
| `handoff-approval-set` | the **set of approval pills** (the component set `.Handoff · Approval`) | `"1"` | `build_approval_set` (`APPROVAL_SET_MARK`) | `find_support`, `read_approval_set`; `generate()` refuses to document a component that carries it (`ErrSupportComponent`) |
| `handoff-organized` | the **user's own component set** (or the single component) in the Component Master | `"1"` | `reflow.rs` `Organize::props`, written together with the wrap (`ORGANIZED_MARK`) | `reflow.rs` `read_set` (`SetInfo.marked`): a rerun that finds the set already fitting still says in the Master that the plugin arranged it |

`kit` is a frame kind that no run makes: it is the Support frame that some Sections of a pre-release 0.11 held inside themselves. It is
recognised by its mark (`Sec::Kit`, `Sec::from_key`) and its set is a candidate for the approval set (`find_support`), never made again.

Nothing else is remembered: **Version** and **Status** are not saved (section 11). The page-level marks are read through the page's
children (`page_info`: at most 2,000 top-level layers, `MAX_TOP_LEVEL`).

## 2. The page's settings (`handoff-settings`)

| Field | Accepted values | Default | Notes | Source |
|---|---|---|---|---|
| `lang` | `en`, `pt-BR` (`pt` is read as `pt-BR`) | `en` | A stored `pt` is rewritten as `pt-BR` by a direct run that saves the settings; the panel leaves it as it is while the language is not changed (`panel.rs` test `a_field_the_user_did_not_change_keeps_its_stored_text_exactly`). A value that names no language is kept as it is. A run reads it **leniently** (`Lang::parse`: anything that is not Portuguese is English); the panel and the direct parameter read it **strictly** (`Lang::strict`). | `direct.rs` `canonical_lang`; `i18n.rs` `Lang::strict`/`Lang::parse`; `section.rs` `generate` |
| `stage` | `light`, `dark`, `auto` (any case) | `auto` | The stage behind the instances (section 5.2). | `store.rs` `StageMode::parse`, `StageMode::default` |
| `on_rerun` | `replace`, `new` (any case) | `replace` | **No effect in 0.11.0.** The panel has no control for it, a direct run still validates it and stores it, and no code of 0.11.0 reads it (`section::generate` never looks at it; the 0.10.0 `gerar::generate`, which did, is not called). What is stored stays as it is. | `store.rs` `Rerun::parse`; `panel.rs` `plan` (comment) |

**Ignored but kept.** Any other key of the stored object goes into `Settings.other`: it is not used and is written back as it was
(`store.rs` `Settings::from_plugin_data`, `to_text`).

| Key | Was (earlier versions) | Now | Source |
|---|---|---|---|
| `tz` | time zone of the generation date | ignored, kept | `store.rs` module doc |
| `line` | colour of the guide lines | ignored, kept | `direct.rs` module doc |
| `fill_opacity` | opacity of the measure highlight | ignored, kept | same |
| `fill` | colour of the measure highlight | ignored, kept | `panel.rs` `plan()` comment |

**Restore defaults** (panel): marks `lang` **and** `stage` as cleared (`panel.rs` `Field::RESTORABLE`) and puts the form back to
English, Auto and "All sections" (`Form::restore_defaults`). Nothing is written until the next documentation is generated; then `plan()` removes
`lang` and `stage` and, when both are marked, empties `other` as well (so `tz`, `line`, `fill`, `fill_opacity` are removed). `on_rerun` is
**not** a field of the panel and stays (test `restore_defaults_removes_the_language_and_the_other_keys_that_are_stored` in `panel.rs`: the stored `{"on_rerun":"new"}` survives).

## 3. The document: one Section, eight frames
### 3.1 The frames

One Section per component (`node.create`, type `section`, named `Handoff · <name>` in both languages), fill `#E7E7EA`, with these frames in
this order (`section.rs` `Sec::ALL`, `Sec::key`, `title_key`). A frame is named `<title> · <name>` (`SectionFrameName`); the Master is named
`Component Master · <name>` (pt-BR `Componente base · <name>`). `<name>` is the set's name for a set, else the component's (`Picked.title`).

| # | `handoff-section` | Title (en) | Title (pt-BR) | Made again by a rerun? | Content |
|---|---|---|---|---|---|
| 0 | `cover` | Cover | Visão geral | **never** (made once; again only if deleted by hand) | 6.1 |
| 1 | `master` | Component Master | Componente base | **every run** (when the Section has one) | 5 |
| 2 | `properties` | Properties | Propriedades | when Update names it | 6.2 |
| 3 | `anatomy` | Anatomy | Anatomia | when Update names it | 7 |
| 4 | `variants` | States and Variants | Estados e variações | when Update names it | 8 |
| 5 | `layouts` | Layouts | Layouts (espaçamento) | when Update names it | 9 |
| 6 | `tokens` | Tokens | Tokens | when Update names it | 6.3 |
| 7 | `changelog` | Changelog | Changelog | **never** (made once; again only if deleted by hand) | 6.4 |

Every frame is made by `new_frame`: an auto-layout frame, vertical, **padding 48**, gap 32, fixed width, height hugs, fill `#F4F4F5`, corner
radius 8, `clip` off, with the two marks of section 1. Each content frame starts with a header (a small grey label with the component's name and
the title at 32 px, bold; `section.rs` `header`) except the Cover.

### 3.2 Geometry

All from `section.rs` (the constants `FRAME_W`, `FRAME_PAD`, `USEFUL_W`, `SECTION_PAD`, `FRAME_GAP`, `FRAME_X`, `SECTION_RIGHT_GAP`, `ORIGIN_Y`; the
test `the_frames_are_in_one_column_with_64_around_and_between_them` fixes them).

| What | Value |
|---|---|
| Width of a frame (`FRAME_W`) | **1180** (wider only when the Master asks, section 5.3) |
| Padding of a frame (`FRAME_PAD`) | **48** on every side |
| Useful width inside a frame (`USEFUL_W`) | 1180 − 2 × 48 = **1084** (`Ctx.useful`: 1084, or the wide frame − 96) |
| Room around the frames inside the Section (`SECTION_PAD`) | **64** (all four sides) |
| Gap between consecutive frames of a column (`FRAME_GAP`) | **64** |
| Column | **one**, top to bottom, every frame at `x = 64` inside the Section; the first at `y = 64` |
| Space between Sections (`SECTION_RIGHT_GAP`) | **256** |
| `y` of the first Section and of the Support frame on the page (`ORIGIN_Y`) | **256** |
| Section width | 64 + the widest frame + 64 = **1308** for 1180-wide frames (`section_size`) |
| Section height | 64 + the lowest bottom of any frame + 64; for a column nobody moved: **64 + Σ heights + 7 × 64 + 64** (eight frames). Example: the Button of the demo, 64 + 6,487 + 448 + 64 = 7,063 |
| Where a Section goes | `x` = the largest right edge of every top-level layer of the page + 256 (0 on an empty page), `y` = 256; "New Section": the `y` of the first Section of the component (`section_origin`) |

The legacy constants of `gerar.rs` (`FRAME_PAD` = 64, `MIN_FRAME_W` = 1280, `SECTION_GAP` = 56) belong to the 0.10.0 drawing and are not used by
the 0.11.0 run (section 17).

### 3.3 Heights are read back

A frame's height is not estimated. After a frame is drawn, `settled_size` reads it (`absoluteBounds` and the `y` and `height` of its children):
when the lowest child bottom + 48 is more than 0.5 px below the frame's bottom (the host had not laid the frame out again), the frame gets
`height = lowest bottom + 48` and `sizingV = fixed`. The frames under it and the Section are placed from that height (`Placed`, `column_ys`,
`section_size`). `scripts/regress.py` `frame_invariants` checks it on every case (section 13.3).

### 3.4 What a run does to the frames already there

`frame_is_made(sec, exists, created, update, master_allowed)` decides, for each frame, in this order:

1. The **Component Master**: made when `master_allowed` (not a "New Section", and either this Section has a Master or no other Section of the
   component holds the original). It replaces the old Master **after** the original component was moved into the new one (deleting a frame
   deletes what is in it).
2. A **permanent** frame (Cover, Changelog) that is **not there**: made (also after a hand deletion). One that is there: **never touched** by any
   `update`, by Generate, or by a language switch.
3. Any other frame that is **not there**: made when the Section is new (`created`) **or when Update names it**: a Properties, Anatomy, … frame deleted by hand
   is made again in its place of the column (the frames under it move so that the one right under starts 64 px below it, a hole the deleted frame left is closed,
   and the Section's height follows the formula of 3.2). When Update does **not** name it, it stays gone and the run raises **`frame_missing`** (`missing_mask`;
   `missing` in the answer lists the kinds). The Master and the permanent frames are never "missing".
4. A frame that **is** there: replaced only when `update` names it (`Update::sections`); otherwise `kept`.

| `update` (panel label en) | Frames it replaces |
|---|---|
| `all` (All sections) | Properties, Anatomy, States and Variants, Layouts, Tokens |
| `tables` (Tables only) | Properties, Anatomy, States and Variants, Layouts |
| `tokens` | Tokens |
| `properties` | Properties |
| `anatomy` | Anatomy |
| `variants` | States and Variants |
| `layouts` | Layouts |

**The first generation makes the eight frames whatever `update` says** (checked: a first run with `update = tokens` returned the eight frames, all
`created`). A replaced frame takes the old one's place; the frames whose top was at or under its old bottom move by the difference in height
(`shift_below`); a frame the person moved up over it is left alone. A frame made again in a Section that lacked it goes under the frame above it
in the column order and the ones at or under that line move down by its height + 64. A frame that failed is rebuilt with only its header and the
error text (`TableFailed`, colour `#C13B30`) and its kind goes into `failed`.

`action` of each entry of `frames`: `created` (new), `replaced` (the old one was deleted), `kept`. The Master is `replaced` on every rerun, also with
`update = layouts`. Run result, Button of the demo, one invocation of the CLI for each run (`scripts/regress.py` does the same):

| Run | Result |
|---|---|
| 1. `{}` | the eight frames `created`, Section made, SVG export added (`export: "ok"`) |
| 2. `update = layouts` | Master and Layouts `replaced`, the other six `kept`, `export: "kept"` |
| 3. `update = tokens` | Master and Tokens `replaced`, the other six `kept` |
| 4. `new_section = true` | a second Section 1308 + 256 to the right, **seven** frames (no Master) all `created`, `master: null`, `created: true`, `rerun: true`, `new_section: true` |
| 5. `lang = pt-BR` | Master and the five content frames `replaced` (in Portuguese), Cover and Changelog `kept` (still in English) |

### 3.5 The Section and its export

A new Section gets one export setting: SVG, `1x`, no suffix (`export.add`, `section.rs` `generate`); `export` in the answer is `"ok"`, the host's error text, or
`"kept"` on a rerun (a rerun does not add it again). A "New Section" gets its own.

### 3.6 The selected layer

`resolve` takes the first selected layer: a **component set** (the documented component is its default variant, the key is the set's id), a
**component** (key = its id), or an **instance** (its main component, with a note about overrides). Any other type is refused with the
dictionary's message (`ErrNotComponent`); an empty selection is `ErrNoSelection`; a component that carries `handoff-approval-set` is refused
(`ErrSupportComponent`); an instance whose original was deleted with its Section is `ErrMainGone`.

**The selected variant.** When the selection is **one variant of a set** (a component whose name has `=` and that a component set holds), or an instance of one,
`resolve` goes **up to the set** and behaves exactly as if the set had been selected: the key is the set's id, the gallery, the Master and the tables are the set's
(`Picked.legacy_key` keeps the variant's id). **Migration:** when no Section is marked with the set's id but one is marked with the variant's id (made by a build
before this correction), that Section is reused as a rerun and its marks (`handoff-report-section` on the Section, `handoff-report` on every frame) are
rewritten with the set's id, so a second Section is not made; `migrated_from` in the answer is the variant's id (else `null`). A component that is in no set is
documented alone, as before.

When several layers are selected only the first is used (`generate` reads `selection.first()`).

## 4. The Support frame and the set of approval pills

`section.rs` `build_support`, `build_approval_set`, `find_support`, `read_area_property`.

* **One set for the whole document.** The set lives in a top-level frame named `Handoff · Support` (pt-BR `Handoff · Apoio`) with the mark
  `handoff-support`: a title, a warning note ("Do not delete: the approval pills on the Covers are instances of the component that lives here.") and the
  set. The set (`COMPONENT_SET`, mark `handoff-approval-set`) is named `.Handoff · Approval` (pt-BR `.Handoff · Aprovação`; the dot keeps it out of a
  published library).
* **Four variants** of one variant property named **`Estado`** (fixed, not translated: it is a property of a component, not a text of the panel):
  `Estado=pendente`, `aprovado`, `ressalvas`, `reprovado`; symbols `…` `✓` `!` `✕`; colours `#6E6E75` `#167A4C` `#9A6410` `#C13B30`
  (`Approval`). Each variant is a pill (290 wide, padding 14, radius 28, gap 12) with a 44 px circle and two texts.
* **Two text properties**, bound to those texts in every variant: **Area** (pt-BR `Área`) and **Name · date** (pt-BR `Nome · data`).
* **When it is made.** Only when a Section is being **made** (the first generation, or "New Section") **and no set exists anywhere in the file**
  (`find_support` looks at every page, at most 500 pages, and at each page's top-level layers: a Support frame, a set standing alone with the mark,
  or the `kit` frame inside a Section). It goes at the page's Section origin (`section_origin`: right of everything, `y = 256`) **before** the
  Section, so the Section stands 256 to its right. A **rerun makes one only when the Cover is made again** (it was deleted by hand) and no set exists: the Support frame is then made the way the
  first generation makes it, 256 px to the **left** of the Section at the Section's `y` (to the right of everything only when something stands there), and the Cover has its pills. If that fails the Cover is made without pills and the run raises **`support_missing`**. A rerun
  that does not make the Cover never makes a Support frame.
* **Which set.** When there are several, the **oldest** (lowest id) is used, so the choice is the same on every run. `support.sets_found` says
  how many there were.
* **Names are the creator's.** The set belongs to the file, not to the language of the run: the Cover's pills are written with the property name the
  **set itself** has. `area_property` takes the first **text** property whose name, without case and without the accents of `á à â ã`, equals
  "Area" in any language of the plugin (`Area`, `Área`, `area`…). The other property name is not read.
* **No Area property** (a set the person changed): the pills are made with no area written and the run raises **`approval_property_missing`**.
* **Writing an area.** Each pill is an instance of the set; `instance.setProperties` writes the area, `component.properties` reads it back, and it is
  written once more when it did not take. `area_retries` is how many pills needed the second write (the first attempt's failure).
## 5. The Component Master

`section.rs` `build_master`, `choose_stage`; `reflow.rs`. The frame holds the original component (or set), **moved** into it
(`node.reparent`): the instances elsewhere in the file stay linked to it. It is the one frame made again on every run.

### 5.1 Content

A header (the label with the component's name, and the title "Component Master", pt-BR "Componente base"), a description, the fixed warning
("Do not delete this frame or the Section: they hold the original component."), when the set carries `handoff-organized` the line "The original set was arranged
with auto layout and wrap.", and a **stage** frame (padding 48, corner radius 12, 1 px border `#D4D4D8`, fill the stage's colour) with the component or set in it.

### 5.2 The stage

The fill is one of three (`theme.rs`): light `#E5E5E5`, mid-grey `#777777`, dark `#14213D`. The page's Stage setting decides:

* `light` or `dark`: that stage; `stage.lowest` is `null`; `no_stage` is never raised.
* `auto` (the default): the stage whose **lowest WCAG contrast** against the colours of the variants is the highest; a tie goes to light, then mid-grey,
  then dark (`contrast::pick_stage3`, `Stage::ORDER`). The colours judged are, for each variant, its own background (its root's solid fill, else the
  set's) or, with none, the colours of its visible elements (`union_colours`). A set of more than **30 variants** (`MAX_STAGE_VARIANTS`) is sampled: the default variant and,
  for each option of each axis, the variant with that option and the other axes at the default (cut evenly if even those are over 30), and
  at most **1,200 layers** are read (`MAX_STAGE_NODES`). `stage.sampled` is `[used, total]` when sampled, else `null`.
* When the lowest contrast of the automatic stage is **under 3:1** (`MIN_STAGE_CONTRAST`) the run raises **`no_stage`**.

**The stage is chosen when the Master or the States and Variants frame is made.** A "New Section" makes no Master but makes the gallery, so it chooses the stage the
same way: `light` or `dark` as the page says, or, in Auto, the best of the three (`stage.sampled`, `stage.lowest` and `no_stage` are then filled as for a Master). If neither frame is
made (an `update` of other frames) the stage is the default (`name: "light"`, `auto: false`) and nothing uses it.
The Stage control of the panel offers Auto, Light and Dark only; mid-grey is reached only by Auto. The same rule (over one variant) picks the
stage of the Anatomy and of the Layouts drawings (this section). `docs/stage-auto.md` describes the rule.

### 5.3 Arranging a wide set (auto layout with wrap)

The room a set has in a 1180 frame is 1180 − 2 × 48 (frame) − 2 × 48 (stage) = **988** (`reflow.rs` `AVAILABLE_W`). `reflow::plan` decides; `organize`
reads, writes in **one** `node.set`, and reads back:

| Plan (`master.plan`) | When | What is written | Master width |
|---|---|---|---|
| `keep` | the set is at most 988.5 wide | nothing | 1180 |
| `organize` | the set is wider, and a line with its widest variant fits in 988 | `wrap: true`, `sizingH: fixed`, `sizingV: hug`, `width: 988`, mark `handoff-organized = "1"`; and, **only when the set had no auto layout**, `layout: horizontal`, `padding: 24`, `gap: 48`, `crossGap: 48` | 1180 |
| `wide` | even one variant does not fit in a line (the widest variant + the set's left and right padding + twice the stroke is over 988), or a single component wider than 988 (also a selected variant of a wide set, section 3.6) | as `organize` with `width` = that line (nothing for a single component or a selected variant) | the width + **192** (= 2 × (48 + 48)) |
| `unsupported` | the set has an auto layout that is **not horizontal** | **nothing** | the set's width + 192 |
| `failed` | the plan was written and the host did not keep it | (written) | the set's width after + 192 |

A set that already has a horizontal auto layout keeps its own padding and gaps (`master.preserved`); only the wrap, the width and the sizing are
written. **A variant is never placed by `x`, `y`.** The read-back (`reflow::check`) wants: a horizontal auto layout, wrap on, fixed width, hugging
height, the asked width, and no variant outside the set; otherwise `plan` becomes `"failed"`, `master.failed` has the reason, and the run raises
**`reflow_failed`** (also for `unsupported`) **and `wide_master`**. In every `wide`, `unsupported` and `failed` case **every frame made in this run** is
as wide as the Master (`max(1180, width + 192)`); the frames kept from earlier runs keep their width, and the Section is as wide as its widest frame.
`master.order_differs` says that the order of the layers is not the order the variants are read in (top to bottom, then left to right); the auto layout
follows the layers.

## 6. The other frames

### 6.1 Cover (made once)

`build_cover`, in this order: the overview label, the title (the component's name, 48 px, bold) and two dashed chips with the guides `v0.0.0` and
`status: to be defined`; a line with a "Last designer" guide on the left and "Updated on" on the right; a "Description" guide (the text a person
types over); **Approvals**: six instances of the set of pills (Designer, Engineering, QA, Writers, PO, PM) in a grid of 3 columns of 350 and 2 rows
(the grid does not follow a wider frame), each with its area written; a legend of the four states (drawn, not components). The person edits the Cover by
hand; no run touches it (the regress case "cover editada" holds this for every `update`).

### 6.2 Properties

One table, `Properties`, columns 240 / 140 / 460 / 244 (= 1084): **Property · Type · Default · Options** (`tables.rs` `build_properties`), from
`component.properties` of the component (for a set, its default variant): the type, the default option, and the options joined by ", " (`—` when
there is one). A component with no properties: the sentence "This component has no properties." and no table. When a frame is wider than 1180
the columns are scaled to the useful width (`reflow::scale_columns`, whole pixels adding up to it).

### 6.3 Tokens

Two tables in one frame (`tokens.rs`): **Tokens by layer** (columns 220 / 320 / 320 / 224: Layer · Colors · Measures · Styles) for the layers of the default
variant that are drawn, identical layers merged; and **Resolved colors** (400 / 342 / 342: Token · Dark · Light), every colour token of every variant once.

* A collection's **Dark** and **Light** columns are its modes **named** dark/escuro/escura and light/claro/clara (case ignored) when it has both; with no
  such pair both columns show its default mode (the first) and the row says "one mode only".
* An alias is followed in the **default mode** of the target's collection, at most **8 hops** (`MAX_HOPS`); a target with more than one mode adds a note; a
  loop, a chain over 8, a missing target or a value that is not a colour is "alias with no value" and is counted in the note "Aliases with no value: n."
* Limits: the two tables share **450 nodes** (`TOKENS_NODE_CAP`); at most **1,000 layers** are read in all the variants (`TOKENS_READ_CAP`; the variants
  not read are said in a note "n variants were not read: their colors may be missing."); rows over the cap are said ("n rows were left out").
* The names of the tables are in the page's language (`Tokens by layer`/`Tokens por camada`, `Resolved colors`/`Cores resolvidas`); `tables[].name` and the
  baseline of the regress carry them.

### 6.4 Changelog (made once)

One table, `Changelog`, columns 160 / 180 / 744: **Version · Date · Change**, with one row of grey guides (`x.y.z`, `Date, typed by hand`, `What changed
in this version.`) for the person to write over. No run touches it.

## 7. Anatomy

`tables.rs` `build_anatomy`; the placer is `gerar.rs` `place_markers` / `place_in_order` (shared with the Layouts). A **stage** (the frame's useful
width, 1084; a 1 px border; at least 240 tall and the instance + 192, grown when the instance's effects or the markers need room, at most 400 px of
effect room per side, `EFFECT_CAP`) with an instance of the default variant in the middle, a **numbered marker** on each part, and the table
**Anatomy** (columns 60 / 120 / 280 / 624: No. · Type · Layer · Note).

* **Numbers.** The layers are grouped (identical layers share a number: the Layer column says `name (xN)` and the text it holds). At most **12** numbers
  (`MAX_BADGES`); the others are left without one and the frame says "+n layers without a number" (`unnumbered`).
* **Markers** are circles of 36 px with the number at 18 px, in the anatomy colour of the stage (light: `#C54600`, dark: `#FCA311`); the guide is a line of 2 px.
* **Placement rules (the contract of the drawing):**
  1. **Only straight guides**: every guide is horizontal or vertical, the marker is in line with the guide's end, never beside it and never at an angle.
  2. Order: the component's own marker first, from the left (then above, below, right); each other layer on its **nearest side**, in the middle of its edge, at the least distance from the
     instance and then every **36 px** farther out (`STAGGER`, 4 levels); then the other positions along the same edge; then the other sides; otherwise **none**.
  3. A place is taken only when: the circle is **at least 12 px** from the other circles (`MARKER_GAP`) and **at least 24 px** from the instance (`MARKER_CLEAR`), and
     its centre is **at least 48 px** from the stage's edge (`MARKER_EDGE`); the guide crosses **no text or vector** of the component other than its own layer or what is
     inside it, stays 4 px from them (`GUIDE_CLEAR`), and does not run along the edge of another layer; its tip is not inside any of them; and guide and circle are not tangled
     with those already placed.
  4. The tip stops **on** the edge of a text and of a layer thinner than 24 px (`SMALL_LAYER`); elsewhere **6 px** in (`GUIDE_INSET`). A container is reached where no layer
     inside it covers its edge.
  5. A layer that got no place is tried again with the left-out ones placed first, up to three more times; the try that keeps the most is used (`rounds`).
* **`anatomy.dropped`** is the list of numbers with no marker. The table's **Note** column says "No marker: it did not fit in the drawing." for each, and the run raises
  **`markers_dropped`**.
* In `scripts/regress.py` the markers are checked (section 13.2).

## 8. States and Variants (a gallery of instances)

`tables.rs` `build_variants`. For each **variant axis** of the set (`variants::axes`, in the order of the properties), a block with the axis name (24 px, bold) and a **grid
of cells** that wraps at the useful width (gap 48 between cells and lines). A cell is a **tile** (the Master's stage colour, padding 24, corner radius 12, at least 160 wide,
as wide as its instance + 48 and at most 1084) holding an **instance** of the variant of that option, and under it the option's name (bold) and a line:

* `default` for the axis's default option;
* `changes: Fill, Border, …` for the others: the **names** of the groups that differ from the default variant (Fill, Border, Opacity, Radius, Spacing, Typography, Visibility,
  Effect, Size, Text, Instance swap), in that fixed order, **at most four** and `…` for more (never a value);
* `label only` when the option differs in nothing the plugin reads.

A second line of text (`closest combination`) when the cell shows not the exact variant but the one that has the option and **the most other axes at the default** (a set with holes), and
`wider than the frame` when the instance is wider than 1084 (it is not scaled). An option no variant has is never a cell: it is listed in "Not defined: …". An axis with only its default option
says "No differences from default". A component with no set or no axis: "This component has no variants."

**The default option of each axis is always in.** The options of the axes then enter **in turn** (rotation, one option of each axis at a time) while the next cell fits in:

* the **node budget** `GALLERY_NODE_BUDGET` = **200** nodes (a cell costs its instance's layers + 4: `CELL_OVERHEAD`), **raised to 3 × the cost of one cell** when one cell alone is over 200 (`gallery_budget`: the "3×" rule);
* **24 cells an axis** (`GALLERY_AXIS_CAP`) and **80 in the frame** (`GALLERY_CELL_CAP`).

The options left out are counted under their axis ("n options left out"). `gallery` in the answer: `cells`, `nodes`, `budget`, `cell_estimate`, `raised` (budget ≠ 200), `kept` (options kept for each axis).
The tiles' fill is the **Master's** stage (`c.stage`).
## 9. Layouts (padding and gap)

`spacing.rs`. The drawings show the component's spacing as **translucent bands** over a copy of the default variant's instance, read from the walk of the
instance (at most 500 layers, `MAX_AUDIT_LAYERS`; hidden layers and what is under them are ignored). The drawing is at **1×**: the host cannot scale an
instance whole (a scale doubles the text but not the auto layout's padding and gap, so the proportions change; measured with `figcraft-cli`), so a small component stays small.

### 9.1 Bands

* **Padding**: up to four bands per layer with auto layout, one per side that has padding (> 0.005): top and bottom as wide as the layer, left and right between them.
* **Gap**: a band between two neighbouring drawn children, only when the layer's auto layout is horizontal or vertical, its gap is set and the space between those children
  **equals** the gap (± 0.5 px): a distribution that spreads the children leaves other spaces, and those are not the gap. A grid has no gap bands.
* Colours: padding **green** `#007D00` (light stage) / `#4ADE80` (dark); gap **violet** `#6D28D9` / `#A78BFA`; neither is the orange of the Anatomy. The fill is 30% for layers at depth 0 and 1 and
  18% for deeper ones; the number's text has at least **4.5:1** against its fill (`MIN_NUMBER_CONTRAST`).

### 9.2 One drawing for each level

A **level** is the depth of a layer with bands in the instance's tree. Levels are numbered in order of depth among those that have bands (Level 1 is the shallowest, usually the component itself; the number is the place in the list, not the raw depth).
Each level is **one drawing** with only its bands, so the bands of a container and of the container inside it never stack. A component with one level makes one drawing without a title; with more,
each drawing has a title `Level N · names` (pt-BR `Nível N · nomes`; 18 px; at most **3 different layer names**, then `…`).

| Limit | Value | Source | What shows |
|---|---|---|---|
| Drawings (levels) per component | **4** | `LEVEL_CAP` | the sentence "Deeper levels not shown." and `spacing.deeper = true` |
| Nodes of all the drawings together | **1,000** (`TOTAL_NODE_CAP`): each level costs its layers' nodes (capped at 120) + the nodes of its copy of the instance; the first level is always shown; a level that does not fit whole is not shown | `shown_levels` | the same sentence |
| Layers in one drawing | **8** (`LAYER_CAP`): the component first, then the layers with the largest area | `choose_layers` | "n layers left out", `left_out` |
| Nodes in one drawing | **120** (`NODE_CAP`): a rectangle for each band and, for each number, at most a guide, a circle and its text; a layer goes in whole or not at all | `choose_layers`, `layer_nodes` | same |

All drawings are on **one stage colour** chosen once (the Anatomy's rule, section 5.2), as wide as the Anatomy's stage (1084), and **share one legend** (Padding / Gap swatches) at the end of the frame.
A component with no layer that has auto layout padding or gap: the frame says "This component does not use auto layout spacing." and `spacing` is `null` (no warning).

### 9.3 Numbers

A **group** is the bands of the same kind (padding or gap), the same value and the same level that are in **one layer**, or in the layers of a **repeated row** (same parent **and same name**: the rows
of a list). Layers with another parent or another name are never joined (a header and a footer with the same padding stay apart). The number is the **value only** ("16", never "(x3)"); whole numbers have no decimals.

* **A group of one band.** The number stands **inside** the band when the band is at least as thick and as long as the patch plus 4 px in **both** axes (the patch is `chars × 8.5 + 8` wide and 18 tall) and no content is under
  the patch. Otherwise **outside**: a circle of 36 px (a pill of `chars × 10.8 + 12` for longer text) with a straight guide, placed by the Anatomy's placer (section 7).
* **A group of two or more bands** gets a **spine**: a vertical line beside the instance with the number at its end and a short horizontal **trace** to each band. Every band goes to the **side** of its centre (left half: left; right
  half: right); a band that crosses the middle (a gap between rows, a padding above or below) goes to the side that has more bands of the group, and on a tie to the left. **Each side has its own spine and its own number**, with the same value ("16" and "16").
  A band with no clean way to the spine of its side tries the other side's spine, and then gets a spine of its own (one trace, one number); only then is it left out.
* Geometry: the first spine is **10 px** from the instance, up to **3** positions **8 px** apart (`SPINE_OFF`, `SPINE_STEP`, `SPINE_SLOTS`); the stub from the spine to its number is **12 px** (`STUB`); a trace goes **6 px** into its band or to its middle when the band is thinner (`TRACE_INSET`); numbers outside are
  at least **12 px** apart.
* **Content** is texts, vectors (vector, ellipse, star, polygon, line, boolean operation) and instances with nothing inside (icons) below the component. A number, a guide, a spine, a stub or a trace **never crosses content, another guide or a number**
  and keeps **3 px** from content (**2 px** at least: `CONTENT_CLEAR`; the guide is 2 px thick). Content a band lies mostly under (a big image) is not an obstacle for the trace that ends in it. A
  retry places first the groups that were left with a band with no trace (up to three more tries); the try that leaves the fewest is used.
* **A band with no number**: it is not drawn with a marker. The frame says "No marker: n bands of v px did not fit in the drawing." (one sentence for each value; "1 band" for one) and the run raises **`spacing_dropped`**
  with the total number of such bands (`spacing.outside_dropped`; `spacing.dropped` is the list of `[value, n]`).

### 9.4 Known limits (kept, not fixed)

Translucent bands over a coloured component change its colour (a green band over a blue button looks wine); the Layouts frame is tall when the instance is tall (the demo's Card: 3,110 px). Both are known and were left as they are.

## 10. Warnings (`warnings`)

`section.rs` `Warn` (`Warn::ALL`, `code`, `text`, `line`). The answer's `warnings` is a list of **codes**, at most one of each, in the order of this table (`Warn::ALL`). A run with none returns `[]`.
The panel shows **one** only (`Message::Warned`): `table_failed`, `frame_missing` or `support_missing` when present (something was not made), else the first and, for the two counted ones, the count from `anatomy.dropped` / `spacing.outside_dropped`. Texts are the dictionary's (`PanelWarn*`).

| Code | Raised when | Where it shows besides the panel | Text (en) |
|---|---|---|---|
| `table_failed` | a content frame could not be made (`failed` has its kind) | that frame: its header and "This section could not be made: {reason}" | A section could not be made: see its frame. |
| `no_stage` | Auto stage and the lowest contrast over the variants (Master) or over the instance (Anatomy) is under 3:1 | nowhere (for the Master the answer's `stage.lowest` shows it; for the Anatomy nothing does) | No stage suits every variant: change it in Stage. |
| `reflow_failed` | the host did not keep the wrap, or the set's auto layout is not horizontal (`unsupported`) | the Master is as wide as the set | The set could not be arranged in lines: the Master was made as wide as it. |
| `wide_master` | the plan is `wide`, `unsupported` or `failed` | every frame made in this run is wider | Wide component: every frame was made wider. |
| `legacy_doc` | first generation and a 0.10.0 frame (`handoff-report`, on the page, no Section) of this component is there | nowhere (it is left as it was) | A 0.10.0 documentation was found and left as it was. |
| `approval_property_missing` | the file's set of pills has no text property named like the area | the Cover's pills have no area written | The approval pills have no Area property: the Cover was made without the areas. |
| `support_missing` | the Cover was made again, no set of pills existed and the Support frame could not be made | the Cover has no pills | The support frame could not be made: the Cover was made without approval pills. |
| `frame_missing` | a frame (not Cover, Changelog or Master) is gone and `update` does not name it; `missing` lists the kinds | nowhere (the frame stays gone) | Not made, because it is missing and Update does not name it: {names}. (several: "they are missing … name them") |
| `markers_dropped` | `anatomy.dropped` is not empty | the Note column of the Anatomy table, per number: "No marker: it did not fit in the drawing." | {n} marker(s) did not fit in the drawing; see the Note column. |
| `spacing_dropped` | `spacing.outside_dropped` is not zero | one note under the legend of Layouts for each value | {n} spacing band(s) did not fit in the Layouts drawing; see the note in that frame. |

## 11. Parameters

### 11.1 Of the direct run (no panel)

A script runs the command with `direct: true` (any other value, or no `direct`, opens the panel). Source: `direct.rs` `wanted`, `run`, `page_from_params`; `section.rs` `generate`.

| Parameter | Accepted | Effect |
|---|---|---|
| `direct` | `true` (boolean) | Skip the panel. |
| `lang`, `stage`, `on_rerun` | the values of section 2; empty keeps what is stored; `-` clears it; anything else is an error with the dictionary's message (`ErrBadLang`, `ErrBadStage`, `ErrBadRerun`) | Written to the page's settings (only if they changed), **before** the run. `on_rerun` is stored and ignored. |
| `update` | one of `all`, `tables`, `tokens`, `properties`, `anatomy`, `variants`, `layouts`; anything else is `all` | Which frames a rerun replaces (section 3.4). Not stored. |
| `new_section` | `true` (boolean) | Make another Section beside the first, with every frame but the Master. With no Section yet it is an ordinary first generation (`new_section` in the answer is then `false`). |
| `version`, `status` | any | **Accepted and ignored** (0.11.0 has no header to write them in; the Cover has guides). |
| `line`, `fill_opacity` | anything | Ignored. |
| `fail_table` | a frame kind | **Only in a build with the `test-hooks` feature** (never in `dist/`): forces that frame to fail. |

The selection is the run's: the first selected layer (section 3.6).

### 11.2 Of the panel

`panel.rs`. The panel's controls (always eleven, in this order): a note, **Language** (`lang`), **Stage** (`stage`: Auto, Light, Dark), a note, **Update** (`update`: the seven options of section 3.4), a note, and the buttons
**Generate** (`generate`), **New Section** (`new_section`), **Restore defaults** (`defaults`), **Close** (`close`), then the message. **Update is not remembered** (it is All each time the panel opens). Generate: refuses (a note, nothing changed) when
nothing is selected or the first layer is not a component, a component set or an instance; writes the page settings that changed; calls `generate` with `{update, new_section}`; moves the view to the start of the Section (`view.set`; a failure is
reported in `view.error` and is not fatal); selects the **holder** (the set or component now in the Master).

## 12. What generate returns (JSON)

Source: the `Ok(json!({...}))` at the end of `generate()` in `section.rs`, and each frame's `Ctx` slot (`Ctx.master`, `markers`, `gallery`, `spacing`, `tokens`, `tables`). This is what a **direct** run returns in `output`;
the CLI wraps it as `{command, plugin, output, logs, calls, fuel, panel}` (the `calls` and `fuel` are the CLI's, section 14). The 0.10.0 keys (`stage` as an id, `anatomy_row`, `legend`, `instance`, `rects`, `labels`, `markers`, `badges`, `axes`, `effects`, `table`, `export` as an object, …) are **not returned any more**.
Types: "id" is a FigCraft layer id (number). The example is a real run (the demo's `Button`), trimmed.

### 12.1 Top-level fields

| Field | Type | Meaning |
|---|---|---|
| `name` | string | The set's name for a set, else the component's (`Picked.title`). |
| `frame`, `section` | id | The **Section** (the same id twice; `frame` is kept for the panel). |
| `created` | boolean | The Section was made in this run (first generation or "New Section"). |
| `rerun` | boolean | A Section of this component already existed (`!first_run`); also true for a "New Section". |
| `new_section` | boolean | A second Section was made beside the first. |
| `update` | string | The `update` used (`all` when absent or unknown). |
| `x`, `y` | number | The Section's position on the page. |
| `frame_size` | `{w}` | The Section's width (same as `view_w` and `section_size.w`). |
| `view_w` | number | The Section's width (the panel's view uses it). |
| `section_size` | `{w, h}` | The Section's size (section 3.2). |
| `tallest` | number | The height of the tallest frame made or kept. |
| `frames` | array | One entry per frame of the column, in order: `{key, id, x, y, w, h, action}` (`action`: `created`, `replaced`, `kept`; `x`, `y` inside the Section). |
| `master_moved` | id or null | The component or set moved into the new Master; null when no Master was made. |
| `holder` | id | The layer that holds the component: the set for a set or a variant, else the component. |
| `migrated_from` | id or null | The id of the variant a reused Section was marked with (3.6); null otherwise. |
| `missing` | array of string | Kinds of frames that are gone and that `update` does not name (the `frame_missing` warning). |
| `component` | id | The **key** of the documentation (the set's id for a set, the variant's or component's id otherwise); **not** the default variant (it was in 0.10.0). |
| `pill_set` | id or null | The set of approval pills used by the Cover. |
| `support` | object | `{sets_found: n, made: {frame, set, x, y, w, h} or null, set: id or null}` (section 4). |
| `area_retries` | number | Pills whose area did not read back and was written again. |
| `warnings` | array of string | The codes of section 10. |
| `failed` | array of string | Kinds of frames that could not be made. |
| `export` | string | `"ok"`, `"kept"` (rerun) or the host's error text. |
| `lang` | `"en"` or `"pt-BR"` | The page's language used. |
| `stage` | object | The **Master's** stage: `{name: "light", "mid" or "dark", dark: boolean, auto: boolean, lowest: number or null, fill: "#RRGGBB", sampled: [used, total] or null}`. |
| `master` | object or null | See 12.2; null when no Master was made. |
| `anatomy` | object or null | See 12.3; null when the Anatomy frame was not made or failed. |
| `gallery` | object or null | See 12.4; null when the frame was not made, failed, or the component has no variants. |
| `spacing` | object or null | See 12.5; null when the frame was not made, failed or the component uses no auto layout spacing. |
| `tokens` | object or null | See 12.6. |
| `tables` | array | `{name, rows}` of each table made, in the page's language (Properties, Anatomy, Tokens by layer, Resolved colors; one per table, not per frame). |
| `host_calls` | number | Host calls the plugin made after reading the selection and the page. |

```json

{
  "name": "Button",
  "frame": 981,
  "section": 981,
  "created": true,
  "rerun": false,
  "new_section": false,
  "update": "all",
  "x": 4920.0,
  "y": 256.0,
  "frame_size": {"w": 1308.0},
  "view_w": 1308.0,
  "tallest": 1441.0,
  "section_size": {"h": 7063.0, "w": 1308.0},
  "master_moved": 629,
  "warnings": ["no_stage"],
  "export": "ok",
  "lang": "en",
  "component": 629,
  "holder": 629,
  "pill_set": 980,
  "area_retries": 0,
  "host_calls": 1225,
  "failed": []
}
```


`frames[]`, the first two of the Button's run (the rest are alike):

```json
[
  {
    "action": "created",
    "h": 705.0,
    "id": 982,
    "key": "cover",
    "w": 1180.0,
    "x": 64.0,
    "y": 64.0
  },
  {
    "action": "created",
    "h": 1363.0,
    "id": 1056,
    "key": "master",
    "w": 1180.0,
    "x": 64.0,
    "y": 833.0
  }
]
```


### 12.2 `master`

`plan` (`keep`, `organize`, `wide`, `unsupported`, `failed`), `frame_w` (the width of the frames made), `organized` (written and kept in this run), `marked` (the set carries `handoff-organized`), `width`, `height` (the set after), `rows` (number of lines), `per_row`
(variants in each line, top to bottom), `variants`, `order_differs`, `failed` (the reason, or null), `preserved` (`{padding, gap, cross_gap}` of a set that had its own auto layout, else null).

```json
{
  "failed": null,
  "frame_w": 1180.0,
  "height": 922.0,
  "marked": true,
  "order_differs": true,
  "organized": true,
  "per_row": [7, 6, 6, 6, 5, 7, 6, 6, 6, 5],
  "plan": "organize",
  "preserved": null,
  "rows": 10,
  "variants": 60,
  "width": 988.0
}
```


### 12.3 `anatomy`

The coordinates are those of the **temporary stage** the drawing is made on (far from the page: `x` near −30,000, `y` from 4,000), not those of the frame; the regress reads them.
`markers[]`: `n`, `centre` and `end` (the circle and where the guide leaves it), `tip` (where the guide ends), `layer` (`[x, y, w, h]` of the layer it points at), `way` (`Root` for the component's own, else `Straight`), `side` (`Top`, `Bottom`, `Left`, `Right`), `shared_edge` (the guide ends on an edge another layer shares);
`rounds[]`: what each round of placing tried (`items`: `[n, kind]` with kind `Text`, `Small` or `Other`; `kept`: the numbers kept); `dropped`: numbers with no marker; `stage`, `component`: `[x, y, w, h]`; `texts`: the text boxes of the instance.

```json
{
  "markers": [
    {
      "centre": [-29550.0, 4144.0],
      "end": [-29508.0, 4144.0],
      "layer": [-29508.0, 4126.0, 100.0, 36.0],
      "n": 1,
      "shared_edge": false,
      "side": "Left",
      "tip": [-29502.0, 4144.0],
      "way": "Root"
    },
    {
      "centre": [-29484.0, 4084.0],
      "end": [-29484.0, 4136.0],
      "layer": [-29492.0, 4136.0, 16.0, 16.0],
      "n": 2,
      "shared_edge": true,
      "side": "Top",
      "tip": [-29484.0, 4136.0],
      "way": "Straight"
    }
  ],
  "rounds": [
    {
      "items": [[1, "Other"], [2, "Small"], [3, "Small"], [4, "Small"], [5, "Text"]],
      "kept": [1, 2, 3, 4, 5],
      "round": 0
    }
  ],
  "dropped": [],
  "stage": [-30000.0, 4000.0, 1084.0, 288.0],
  "component": [-29508.0, 4126.0, 100.0, 36.0],
  "texts": [[-29468.0, 4134.0, 44.0, 20.0]]
}
```


### 12.4 `gallery`

`cells` (drawn), `nodes` (made), `budget`, `cell_estimate` (nodes of one cell), `raised` (budget ≠ 200), `kept` (cells of each axis).

```json
{
  "budget": 200,
  "cell_estimate": 11,
  "cells": 12,
  "kept": [4, 3, 5],
  "nodes": 120,
  "raised": false
}
```


### 12.5 `spacing`

Totals: `levels` (levels with bands), `drawings` (shown), `deeper` (levels left out), `left_out` (layers left out), `with_auto_layout` (layers with auto layout), `groups`, `inside`, `outside`, `spines` (numbers), `outside_dropped` (bands with no number), `dropped` (`[value, n]`), `nodes`, `dark` (stage), and
`list`: **one object for each drawing**: `title` (null for a single level), `depth`, `layers`, `left_out`, `stage` and `component` (`[x, y, w, h]`, temporary stage), `bands`, `groups`, `inside`, `outside`, `spines` (an object for each spine: `label`, `left`, `bands`, `traces`, `line`), `nodes`,
`list` (each band: `layer`, `depth`, `gap`, `value`, `rect`), `markers` (the outside numbers of a single band: `n`, `centre`, `tip`, `layer`), `badges` (each number: `label`, `inside`, `rect`), `guides` (every guide, spine, stub and trace as `[x1, y1, x2, y2]`), `content` (the boxes of texts, vectors and icons).
Here the `list` of the example is cut to one object of each kind.

```json
{
  "dark": false,
  "deeper": false,
  "drawings": 1,
  "dropped": [],
  "groups": 3,
  "inside": 0,
  "left_out": 0,
  "levels": 1,
  "nodes": 14,
  "outside": 1,
  "outside_dropped": 0,
  "spines": 4,
  "with_auto_layout": 2,
  "list": [
    {
      "title": null,
      "depth": 0,
      "layers": 1,
      "left_out": 0,
      "stage": [-30000.0, 7000.0, 1084.0, 240.0],
      "bands": 5,
      "groups": 3,
      "inside": 0,
      "outside": 1,
      "spines": [
        {
          "bands": 2,
          "label": "8",
          "left": true,
          "line": [-29518.0, 7070.0, -29518.0, 7106.0],
          "traces": 1
        }
      ],
      "nodes": 14,
      "list": [
        {
          "depth": 0,
          "gap": false,
          "layer": 0,
          "rect": [-29508.0, 7102.0, 100.0, 8.0],
          "value": 8.0
        }
      ],
      "markers": [
        {
          "centre": [-29472.0, 7060.0],
          "layer": [-29476.0, 7110.0, 8.0, 20.0],
          "n": 1,
          "tip": [-29472.0, 7110.0]
        }
      ],
      "badges": [
        {"inside": false, "label": "8", "rect": [-29490.0, 7042.0, 36.0, 36.0]},
        {"inside": false, "label": "8", "rect": [-29566.0, 7052.0, 36.0, 36.0]}
      ],
      "guides": [[-29472.0, 7078.0, -29472.0, 7110.0], [-29518.0, 7070.0, -29518.0, 7106.0]],
      "content": [[-29490.0, 7114.0, 12.0, 12.0], [-29468.0, 7110.0, 44.0, 20.0]],
      "component": [-29508.0, 7102.0, 100.0, 36.0]
    }
  ]
}
```


### 12.6 `tokens`

`layers` (rows after merging), `layer_rows`, `colour_rows`, `left_out` (`[rows of the first table, of the second]` left out by the node cap), `read_layers`, `variants`, `variants_read`, `mapped` (colour rows with a Dark/Light pair), `one_mode`, `style` (rows coloured by a style),
`loose` (values with no token), `alias_multi` (aliases to a target with several modes), `broken` (aliases with no value), `collections`.

```json
{
  "alias_multi": 0,
  "broken": 0,
  "collections": 3,
  "colour_rows": 17,
  "layer_rows": 3,
  "layers": 3,
  "left_out": [0, 0],
  "loose": 0,
  "mapped": 17,
  "one_mode": 0,
  "read_layers": 360,
  "style": 0,
  "variants": 60,
  "variants_read": 60
}
```


### 12.7 `support` and `tables`

```json
{
  "support": {
    "made": {"frame": 952, "h": 331.0, "set": 980, "w": 1368.0, "x": 3296.0, "y": 256.0},
    "set": 980,
    "sets_found": 0
  },
  "tables": [
    {"name": "Properties", "rows": 7},
    {"name": "Anatomy", "rows": 5},
    {"name": "Tokens by layer", "rows": 3},
    {"name": "Resolved colors", "rows": 17}
  ]
}
```


### 12.8 `stage`

```json
{
  "auto": true,
  "dark": true,
  "fill": "#14213D",
  "lowest": 1.2346731638053356,
  "name": "dark",
  "sampled": [10, 60]
}
```


### 12.9 What the panel returns after **Generate** or **New Section**

Source: `panel.rs` `generate_click`, `show_start`.

| Field | Type | Meaning |
|---|---|---|
| `phase` | `"generate"` | Which callback ran. |
| `refused` | boolean | `true` when nothing was generated (no selection, or the first layer is not a component, a set or an instance). |
| `message` | string | The message the panel shows, in the page's language. |
| `frame` | id | The Section (only when `refused` is `false`). |
| `view` | object | The view after moving to the start of the Section: `{zoom, cx, cy, vw, vh, asked: {zoom, cx, cy}}`, or `{error: string}` when it did not move (not fatal). |
| `selected` | id | The holder, selected after the run. |
| `new_section`, `update`, `frames`, `warnings`, `stage` | | The same as in 12.1. |

The other callbacks answer `{phase: "open", lang, message}` (the menu), `{phase: "close"}` (Close) and `{phase: <control>, lang, message, selection_now}` (a field edit or Restore defaults); each redraws the panel in the form's language.
## 13. The regression net (`scripts/regress.py`, `scripts/regress.sh`)

It runs `generate` headless (`figcraft-cli run`, one invocation for each run; everything is written to a temporary folder) on a fixed set of cases and compares what comes out with `tests/baseline.json`
(**28 public cases**: the four fixtures of `tests/fixtures`, five components of `demo/ds-base-demo.figcraft` found **by name** each time, five pt-BR runs (two fixtures and three demo components), and the cases "cover editada" ×10,
"apoio compartilhado", "master / card", "master / wide", "2º componente"). `--private` runs the cases of `tests/private/` (a folder git ignores; **22 cases**) against `tests/private/baseline-private.json`.
`--update [text]` rewrites the baseline from the current build (only the cases whose name contains `text`); `--dump FILE` writes the results and compares nothing; `--baseline FILE` compares with another file.

### 13.1 What a case records

| Key | Type | What it is |
|---|---|---|
| `frames` | `[[key, x, y, h], …]` | the frames of the Section (`x`, `y` inside it) |
| `section` | `[w, h]` | the Section's size |
| `tables` | `[[name, rows], …]` | the tables made |
| `warnings`, `failed` | array of string | as in the answer |
| `markers` | object or null | the Anatomy's numbers (13.2); null with no marker |
| `spacing` | object or null | the Layouts' numbers (13.2); null with no `spacing.list` |
| `fuel`, `calls` | number | the CLI's, **synthetic** (section 14); a difference over **5%** (`FUEL_TOLERANCE`) is reported |
| `export` | string | not in the baseline: must be `ok` (`kept` is accepted in a private file that already holds a Section) |
| `invariants` | array | not in the baseline: the violations of 13.3 |

No ids and no texts are recorded. A baseline entry in the old format (without `frames`) is reported as old and asks for `--update`.

### 13.2 Invariants of the drawings (any non-zero value fails the run)

**Anatomy** (`marker_check`): `count`, `edge_ok` (the nearest centre is at least 48 px from the stage's edge), `min_gap` (least gap between two circles, **at least 12**), `root_straight`, `dropped`, `oblique` (guides that are not horizontal or vertical: **0**),
`tip_in_text` (guide tips inside a text: **0**), `cross` (guides crossing another guide or running over another text: **0**). `count`, `edge_ok`, `root_straight` and `dropped` are compared with the baseline; the others fail the run.

**Layouts** (`spacing_check`, over every drawing of `spacing.list`): `levels`, `drawings`, `deeper`, `groups`, `inside`, `outside`, `spines`, `dropped` are compared with the baseline; these fail the run when not zero: `badge_over_content` (a number whose patch or marker meets
content), `guide_over_content` (a guide, spine or trace within **1.9 px** of content: the guide is 2 px thick), `cross` (guides that cross each other), `oblique`, `stack_overlap` (bands of two layers over each other in one drawing),
`spine_cross` (a guide, spine, trace or stub through a number; a stub touching its own number does not count); and `min_gap` (the least gap between two numbers outside) must be **at least 12**.

### 13.3 Invariants of the frames (`frame_invariants`)

Read back from the file after the run: (a) the bottom of a frame's content is at most its bottom − 48; (b) the space between consecutive frames is exactly **64** (± 0.5); (c) no frame overlaps another; (d) the Section's height is the formula of section 3.2.
The other cases ("cover editada", "apoio compartilhado", "master /", "2º componente") record their own results and are compared whole.

## 14. Fuel, nodes and what is measured where

* **`fuel` and `calls` of the CLI are synthetic.** They are the instruction count and the number of host commands as `figcraft-cli 0.18.1` counts them in a headless run (`plugin.run` answers `{output, logs, calls, fuel, panel}`); the app was not used to measure them.
  `output.host_calls` is a different count: the host calls **the plugin** made after reading the selection and the page (for the demo's Button: 1,225 against `calls` 3,696; the difference is not explained in the source).
* **Limit of a run.** `plugin.run` takes an optional `fuel`: a **lower** instruction budget. A run that exhausts it fails with an "instruction budget" error and the host rolls everything back (`scripts/panel_sequence.py` forces it with `fuel: 9000000`). The host's own default limit
  is not documented in this repository.
* **Measured now (synthetic, this HEAD):** the demo's Button 358.7 M, the demo's Card 396.7 M, a variant of the Toast fixture (`copia-2`, id 3) 120.7 M, the heaviest public case (fixture `carga-v2`, id 194) 820.6 M.
* **Nodes.** The caps that keep a run small: Layouts 120 nodes and 8 layers a drawing, 4 drawings, 1,000 nodes in all (section 9.2); the gallery 200 nodes (or three cells), 24 cells an axis, 80 in the frame (section 8); Tokens 450 nodes and 1,000 layers read (6.3); the
  Anatomy 12 numbers (section 7), 250 lines of the audit and 500 layers walked of an instance (`MAX_AUDIT_LINES`, `MAX_AUDIT_LAYERS`); the stage of the Master 30 variants and 1,200 layers (5.2); the page's top-level layers read: 2,000 (`MAX_TOP_LEVEL`). Each layer read is a host call.
* **What only the app measures:** real text widths (the code assumes 8.5 px a character in a Layouts patch and 10.8 px in a marker), the final heights the host lays out (the plugin reads them back, `settled_size`), and the panel in the app's themes.

## 15. Language

* **One language for each file.** The page's `lang` (section 2) is read at the start of every run and **every** text the plugin writes comes from it: the frames' names, the layer names, the tables, the notes, the Cover and Changelog guides (`i18n.rs`, `locales/en.json`, `locales/pt-BR.json`,
  335 keys each; a key a language leaves out falls back to English, and the test `pt_br_misses_no_key` keeps that from happening; `docs/i18n-strings.md` lists them and is regenerated with `I18N_WRITE=1 cargo test i18n`). The panel's language follows its Language control, and a switch redraws it.
* **What a language switch does to a document that exists.** A rerun in the other language replaces the frames "Update" names **and the Master** in the new language; **Cover and Changelog are never replaced, so they stay in the old language** (checked: a rerun with `lang = pt-BR` of a document made in English returned both `kept`). The names of the frames
  kept keep their old language.
* **The Support frame** is the exception that makes this safe across languages: the Cover's pills are written with the property name the set **already has** (section 4), and the variant property `Estado` is the same in both languages.
* An unknown `lang` in the page's settings reads as English; an unknown `lang` **parameter** of a direct run is an error (`language must be en or pt-BR, not "xx"`) and nothing is written.

## 16. History

### 16.1 `handoff-meta`

Earlier versions saved a `handoff-meta` sheet on each component (version, status, default variant, and a typed `date` before 0.9.6). The plugin **does not read it and does not delete it**; it stays in the file as it was (`store.rs` module doc).

### 16.2 From 0.9.6 to 0.10.0 (kept for the files made by those versions)

| Where | 0.9.6 | 0.10.0 |
|---|---|---|
| `generated` (`date`, `created`, `tz`, `failure`, `leftover_comment`) | present | removed (no date) |
| `settings.line` | the line colour | removed |
| `settings.fill` | an adjustable fill colour | only informative |
| Page `handoff-settings`: `line`, `fill`, `tz`, `fill_opacity` | settings fields | ignored and kept in `other` |
| Page `handoff-settings`: `lang`, `on_rerun`, `stage` | same fields | same fields |
| Frame `handoff-report` | the component's or set's id | same |
| Manifest | `api` 1, `comment.*` permissions, a `settings` command | `api` 2, `view.get`, `view.set`; no `settings` command |

The 0.10.0 answer's other fields are listed in the 0.10.0 version of this file (`git show v0.10.0:docs/data-contract.md`); 0.11.0 does not return them (see the list at the top).

## 17. Known inconsistencies and what is not confirmed

Facts of the code that this document does not change. (Four found in the first version of this file, the selected variant, the frame deleted by hand, the rerun with no Support frame and the stage of a "New Section", were corrected afterwards and are described above as they are now.)

1. **`on_rerun`, `version` and `status`** are accepted by a direct run (and `on_rerun` is stored) and have no effect (2, 11). `direct.rs`'s module doc still says `version` and `status` go to the header.
2. **The panel shows only the first warning** when there are several (10).
3. **Constants of the old drawing**: `gerar.rs` keeps the 0.10.0 constants (`FRAME_PAD` 64, `MIN_FRAME_W` 1280, `SECTION_GAP` 56) next to `section.rs`'s (48, …); they are used only by the 0.10.0 `generate`, which nothing calls.
4. **Two `plugin.run` calls in one CLI script** do not see the Section the first one made: the second run made a second Section (`created: true`, `rerun: false`). `scripts/regress.py` runs each generation in its own invocation (the script is "one all-or-nothing undo step"); the cause is not confirmed.
5. **`component` in the answer is the key**, not the default variant of a set (it was the default variant in 0.10.0).

**Not confirmed with a source or a run** (each is said where it appears): the host's own fuel limit and the difference between `calls` and `host_calls` (14); the text widths and the final heights in the app (14); the exact unit of `rounds` (what a "round" is beyond "an attempt of the placer") (12.3); that the panel's message fits in three lines in the
app (`panel.rs` tests check it on the host); the failure path of `support_missing` (it needs a host that refuses the Support frame; it is not run).
