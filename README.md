# Handoff Components

A plugin for FigCraft that documents the selected component, or component set, as handoff documentation on the same page.

![Documentation for the Code component](docs/samples/report-code.png)

> Handoff Components is an independent, community-made plugin for FigCraft. It is not affiliated with, endorsed by, or supported by the ArtCraft team. FigCraft and ArtCraft are names and trademarks of their respective owners.

## What it does

Select a component or a set and run one command. The plugin writes the documentation next to your work, with:

- **Anatomy**: the layers of the component, numbered, with their properties and tokens. Each property is audited:
  bound to a variable or style, loose ("no token"), missing from the file, or suspect (bound, but to something of
  another value).
- **Spacing**: gap and padding, drawn over the component.
- **Variant differences**: for a component set, what each variant changes from the default one, axis by axis.
- **Same page, one click**: the documentation goes to the right of everything on the page, at the same height. (Inside the file the frame keeps the old pluginData key `handoff-report`, so a rerun still replaces documentation made by earlier versions.)

More samples: [Button](docs/samples/report-button.png), [Card](docs/samples/report-card.png),
[Accordion](docs/samples/report-accordion.png).

## Requirements

- FigCraft, [a design app from the ArtCraft team](https://getartcraft.com/apps/figcraft), and `figcraft-cli` on the PATH.
- Tested with **figcraft-cli 0.17.0**. The plugin SDK is `figcraft-plugin-sdk` at tag `v0.14.0`.
- Only to build the plugin: Rust 1.95 (`rust-version = "1.95"` in `Cargo.toml`) with the `wasm32-unknown-unknown` target,
  for example with rustup:

  ```sh
  rustup toolchain install 1.95
  rustup override set 1.95          # run inside this repository
  rustup target add wasm32-unknown-unknown
  ```

  The build downloads the SDK from GitHub (a git dependency) and the other crates from crates.io.
- Python 3, only for the regression net.

## Install and build

```sh
scripts/build.sh
```

It writes `dist/` and prints the absolute path of the manifest to import. In FigCraft:
**Plugins ▸ Development ▸ Import plugin from manifest…** (in the Portuguese app: **Plugins ▸ Desenvolvimento ▸ Importar plugin a partir do manifesto…**), pick `dist/figcraft-plugin.json`.

## Usage

Select a component or a set, then run **Plugins ▸ Handoff Components ▸ Generate documentation**.

**Plugins ▸ Handoff Components ▸ Settings…** opens a panel (it needs a FigCraft that supports plugin API 2). The panel has two parts of fields, then the messages and the buttons:

- **This page**, saved on the current page: language (English or Português (Brasil)), line colour, fill (a box *Fill: line colour at 30%*; unchecked, a fill colour appears), stage (light, dark or auto) and what a rerun does (replace or new).
- **The selected component**, saved on it (on its set, for a variant): version, status and, for a set, one menu per variant property that makes up the default variant (*(none)* leaves the property out). The section is always there: with no component loaded it says so (the app does not tell a plugin that the selection changed, so select a component and press **Reload**).
- **Messages**, below the fields: whether there are unsaved changes (`●`), whether everything is saved (`✔`) and the result of the last button (`✔` done, `✖` refused). The buttons come after all the fields, on purpose: the app hands a panel's events to the plugin in the order of its controls, so a button above a field would run before the field's new value arrived.

Nothing is saved until you press **Save changes**; the panel says whether there are unsaved changes, and stays open after saving. **Reload** reads the selection and what is saved again (and discards unsaved changes); **Restore defaults** sets the page's fields to "not defined" (press **Save changes** to remove them); **Close** closes it. Notes:

- **The component is the one loaded in the panel.** Saving writes to it, and refuses (saving nothing) when the selection is another one: press **Reload**. This also happens after you generate the documentation with the panel open, because the documentation becomes the selection.
- **Empty version or status erases it.** The saved values appear filled in.
- **Only what you changed is written.** A field you did not change keeps exactly what is saved, and a setting that is not defined stays not defined: choosing the default value by hand (for example the default line colour) while nothing is saved does not save it. Use **Restore defaults** to go back to "not defined" for the page.
- **One undo step.** **Save changes** writes the page and the component together as one undo step, and none when nothing changed. Undoing does not refresh the panel: press **Reload**.
- The panel is in the page's language and redraws at once when you change it, keeping what you typed.

The defaults are: language English; line colour `#FCA311`; fill colour the line colour at 30%; stage `light`; on rerun `replace`; default variant the component's own default, else the first variant of the set; version and status none (left out of the documentation).

**Scripts** can still set everything without the panel: `plugin.run` with the `settings` command and parameters `lang`, `cor_linha`, `cor_preenchimento`, `on_rerun`, `stage`, `versao`, `status` and `default_variant` writes them directly (an empty parameter keeps the saved value, `-` clears it), with the same checks and messages as before; this is what `scripts/regress.sh` uses for its pt-BR cases.

### The documentation's date

The documentation does not show a generation date. The plugin host gives a plugin only the instant in UTC, with no time zone and
no local time, so a date would be wrong for many people; it will come back when the plugin API offers a clock.

- **Old settings.** Earlier versions (0.9.6 and before) saved a `tz` (time zone) on the page, and 0.9.5 and before a typed `date` in
  each component's sheet (`handoff-meta`). Both are ignored, not erased: they stay in the file as they were. **Restore defaults**
  removes the page's keys, `tz` included.
- **A script that passes `tz`** to the `settings` command: the parameter is ignored.

## Demo

[`demo/`](demo/README.md) holds the **DS Base Demo**: a small neutral design system built from scratch
(Icon, Button, Card, Accordion, Code), with variables, styles and Light/Dark modes. Open
`demo/ds-base-demo.figcraft` and generate documentation on it.

## Known limitations

- Version 0.9.x, before 1.0: expect rough edges.
- The documentation is available in English (the default) and Brazilian Portuguese (`pt-BR`), chosen per page in **Settings…**. The **Settings…** item in the Plugins menu stays in English (the host takes its label from the manifest); the panel itself follows the page's language, and the values it saves stay in English (`light`, `dark`, `auto`, `replace`, `new`). In `pt-BR`, a few words are still English because no translation has been agreed: the type of a variable in a token conflict (`number`, `color`…); the name of an effect the plugin doesn't describe (noise, texture, glass); and the technical reasons the host gives in an error.
- The host does not report a layer's text case, so it is missing from the typography lines.
- A run has a fixed instruction budget (1,000,000,000 by default in the FigCraft v0.14.0 sources). Very large components can
  use much of it: the heaviest case in the regression net, a test fixture with a 155-layer component, uses 64.2%; the DS Base
  Demo's Card uses 23.5%.
- Swapping an instance between variants shows up as one layer becoming hidden and another visible.
- Long text changes are shown as a short excerpt (40 characters) around the first difference.
- Layer rotation is not audited or compared between variants.
- The app's Arrow is a Line with an arrowhead (the host reports it as a `LINE`), so the documentation calls it Line (Linha in pt-BR), not Arrow.

## Tests and regression

```sh
cargo test                      # 134 unit tests + 4 that keep English text out of the code
scripts/regress.sh              # 14 cases against tests/baseline.json: 9 in English (4 fixtures + 5 on the DS Base Demo) and 5 in pt-BR
scripts/regress.sh --private    # extra cases on files kept out of the repository (tests/private/), if present
scripts/panel_sequence.py       # the Settings panel driven like the app does: field, then Save, writes on the first click
```

`scripts/regress.sh --update [text]` rewrites the baseline for the cases whose name contains `text` (all, if omitted).
The demo components are found by name on every run. The `--private` option only works on a machine that has a local
`tests/private/` folder; without it, it says so and exits.

## Texts and languages

Every text the plugin shows lives in `locales/en.json` and `locales/pt-BR.json` (a flat `"Key": "text"` object each; `docs/i18n-strings.md`
lists both side by side). They are embedded in the `.wasm`; a key missing from `pt-BR.json` falls back to English. `cargo test` checks that
`pt-BR.json` has every key of `en.json`, with the same `{placeholders}`, and `tests/english_literals.rs` fails on English text written
in the code outside the dictionary (exceptions, each with its reason, are in `tests/literals-allow.txt`).

## License

MIT, see [LICENSE](LICENSE). The demo bundles Phosphor Icons path data and names the Inter and JetBrains Mono fonts, and the
plugin is built from Rust crates with their own licences; their notices are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Credits

Built with Claude Code (Anthropic).
