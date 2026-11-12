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

Run **Plugins ▸ Handoff Components ▸ Generate documentation…**. It opens a panel (it needs a FigCraft that supports plugin API 2) that stays open until you close it:

- **This page**, saved on the current page: language (English or Português (Brasil)), line colour, **Fill opacity (0–100)** (a whole number from 0 to 100, default 30: the fill is always the line colour at that opacity, 0 = no fill; the opacity is absolute, so a line colour with its own alpha does not change it), stage (light, dark or auto) and what a rerun does (replace or new).
- **Documentation**: **Version** and **Status**. Left blank, the documentation shows “to be defined” (“a definir” in Portuguese): the header always has the line `Version: … · Status: …`. Nothing is remembered: after a documentation is made, both fields are blank again.
- **Messages**, below the buttons (so the buttons never move when a message is longer or shorter): what happened the last time (`✔` done, `✖` refused) and a fixed warning (`ℹ`) that the screen freezes while the documentation is generated. The buttons come after all the fields, on purpose: the app hands a panel's events to the plugin in the order of its controls, so a button above a field would run before the field's new value arrived.

**Generate documentation** works on whatever is selected **at the moment of the click**: a component, a component set (it shows the set's own default variant, else the first one) or a variant. Then it:

1. refuses, with a note and without touching the file, when nothing is selected, the selection is not a component, a set or a variant, or the fill opacity is not a whole number from 0 to 100;
2. saves the page's settings as the panel shows them (only what you changed is written);
3. makes the documentation, as one undo step;
4. takes the screen to the **start** of the documentation, as wide as the screen allows (zoom 100% at most);
5. gives the selection back (what was selected before the click stays selected);
6. empties Version and Status.

If something fails on the way, the app puts everything back as it was and shows the error. **Restore defaults** puts line colour, fill opacity, stage and “on rerun” back to the defaults and **keeps the language** (they are saved with the next documentation, and old keys the plugin ignores are dropped then); **Close** closes the panel. The panel is in the page's language and redraws at once when you change it, keeping what you typed.

The defaults are: language English; line colour `#FCA311`; fill 30% (the line colour at 30% opacity); stage `light`; on rerun `replace`.

**Scripts** make the documentation with no panel by passing `direct: true` (the menu sends no parameters, and opens the panel): `plugin.run` with the `generate` command and the parameters `direct`, and optionally `lang`, `line`, `fill_opacity` (a whole number, 0 to 100), `stage`, `on_rerun` (they set the page's settings: an empty one keeps what is saved, `-` clears it, anything else is checked, with the same messages as the panel) and `version`, `status` (the header's texts; absent means “to be defined”). The selection is the one at the time of the call. This is what `scripts/regress.sh` uses.

### The documentation's date, and what older versions saved

The documentation does not show a generation date. The plugin host gives a plugin only the instant in UTC, with no time zone and
no local time, so a date would be wrong for many people; it will come back when the plugin API offers a clock.

- **Old files.** Earlier versions saved a `tz` (time zone) and a `fill` colour on the page, and a `handoff-meta` sheet on each component (version, status, default variant, and a typed `date` before 0.9.6). The plugin ignores them and does not erase them: they stay in the file as they were. **Restore defaults** removes the page's keys, `tz` included.

## Demo

[`demo/`](demo/README.md) holds the **DS Base Demo**: a small neutral design system built from scratch
(Icon, Button, Card, Accordion, Code), with variables, styles and Light/Dark modes. Open
`demo/ds-base-demo.figcraft` and generate documentation for it.

## Known limitations

- Version 0.9.x, before 1.0: expect rough edges.
- The documentation is available in English (the default) and Brazilian Portuguese (`pt-BR`), chosen per page in the panel. The item in the Plugins menu stays in English (the host takes its label from the manifest); the panel itself follows the page's language, and the values it saves stay in English (`light`, `dark`, `auto`, `replace`, `new`). In `pt-BR`, a few words are still English because no translation has been agreed: the type of a variable in a token conflict (`number`, `color`…); the name of an effect the plugin doesn't describe (noise, texture, glass); and the technical reasons the host gives in an error.
- The host does not report a layer's text case, so it is missing from the typography lines.
- The screen freezes while the documentation is generated (it is made inside the click, in the app's own thread, with no progress indicator).
- A run, and each click in the panel, has a fixed instruction budget (1,000,000,000 by default in the FigCraft v0.14.0 sources). Very large components can
  use much of it: the heaviest case in the regression net, a test fixture with a 155-layer component, uses 64.2%; the DS Base
  Demo's Card uses 23.5%.
- Swapping an instance between variants shows up as one layer becoming hidden and another visible.
- Long text changes are shown as a short excerpt (40 characters) around the first difference.
- Layer rotation is not audited or compared between variants.
- The app's Arrow is a Line with an arrowhead (the host reports it as a `LINE`), so the documentation calls it Line (Linha in pt-BR), not Arrow.

## Tests and regression

```sh
cargo test                      # 136 unit tests + 4 that keep English text out of the code
scripts/regress.sh              # 14 cases against tests/baseline.json: 9 in English (4 fixtures + 5 on the DS Base Demo) and 5 in pt-BR
scripts/regress.sh --private    # extra cases on files kept out of the repository (tests/private/), if present
scripts/panel_sequence.py       # the panel driven like the app does: Version and Status, then the button, in one click; refusals; direct mode
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
