# Handoff Components

A plugin for FigCraft that documents the selected component, or component set, as a handoff report on the same page.

![Report for the Code component](docs/samples/report-code.png)

> Handoff Components is an independent, community-made plugin for FigCraft. It is not affiliated with, endorsed by, or supported by the ArtCraft team. FigCraft and ArtCraft are names and trademarks of their respective owners.

## What it does

Select a component or a set and run one command. The plugin writes a report next to your work, with:

- **Anatomy**: the layers of the component, numbered, with their properties and tokens. Each property is audited:
  bound to a variable or style, loose ("no token"), missing from the file, or suspect (bound, but to something of
  another value).
- **Spacing**: gap and padding, drawn over the component.
- **Variant differences**: for a component set, what each variant changes from the default one, axis by axis.
- **Same page, one click**: each report goes to the right of everything on the page, at the same height.

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

Select a component or a set, then run **Plugins ▸ Handoff Components ▸ Generate report**.

**Plugins ▸ Handoff Components ▸ Settings…** sets, among others, the language (`en` or `pt-BR`; a `pt` saved by an earlier version keeps working and means `pt-BR`), the line colour, the stage
(`light`, `dark` or `auto`), what a rerun does (`replace` or `new`), the time zone of the report's date, the default variant,
and the version and status shown on the report. Leave a field empty to keep its current value; type `-` to clear it and go back
to the default. The defaults are: language English; line colour `#FCA311`; fill colour the line colour at 30%; stage `light`;
on rerun `replace`; time zone `-3`; default variant the component's own default, else the first variant of the set;
version and status none (left out of the report).

### The report's date

Every report says when it was generated (`Generated on: 2026-11-11` in English, `Gerado em: 11/11/2026` in Portuguese).
There is no date to type: it is read from the app's clock on each **Generate report**.

- **How.** The plugin host gives a plugin no clock, so the plugin adds a temporary comment (`tmp (handoff-components date)`)
  to the current page, reads the time the app stamped on it, and deletes the comment. This is why the manifest asks for
  `comment.add`, `comment.list` and `comment.delete`; the app doesn't ask for permission when you import a manifest that has
  them. The plugin doesn't count comments before and after, so a comment someone else makes in a live session can't cause a
  false alarm.
- **Time zone.** Set in **Settings…** (field *Time zone*, per page): hours from UTC, from `-14` to `14`, fractions allowed
  (`-3`, `0`, `9`, `5.5`). The default is `-3` (São Paulo). The zone is fixed: there is no daylight saving time, so change it
  by hand if your zone changes. An invalid value is refused with a message and nothing is saved.
- **When it fails.** If the comment can't be added or read, the report is still made, says `Generated on: unavailable`
  (`Gerado em: indisponível`) and shows a warning at the top; no old date is used. If the temporary comment can't be deleted
  (after a second try), the report keeps its date and a warning at the top gives the comment's id, so you can delete it by hand.
- **Old files.** Earlier versions stored a typed `date` in each component's sheet (`handoff-meta`). It is ignored now, but
  not erased: it stays in the file as it was.
- **Not tested.** Shared (collaborative) sessions, and files opened for viewing only (there the comment can't be added, so the
  report should say `unavailable`). Both are untested in the real app; the failure path is tested only by simulation.

## Demo

[`demo/`](demo/README.md) holds the **DS Base Demo**: a small neutral design system built from scratch
(Icon, Button, Card, Accordion, Code), with variables, styles and Light/Dark modes. Open
`demo/ds-base-demo.figcraft` and generate reports on it.

## Known limitations

- Version 0.9.x, before 1.0: expect rough edges.
- The report is available in English (the default) and Brazilian Portuguese (`pt-BR`), chosen per page in **Settings…**. The Settings form labels stay in English (the host shows one label per field), and so do the values you type there (`light`, `dark`, `auto`, `replace`, `new`). In `pt-BR`, a few words are still English because no translation has been agreed: the type of a variable in a token conflict (`number`, `color`…); the name of an effect the plugin doesn't describe (noise, texture, glass); and the technical reasons the host gives in an error.
- The host does not report a layer's text case, so it is missing from the typography lines.
- A run has a fixed instruction budget (1,000,000,000 by default in the FigCraft v0.14.0 sources). Very large components can
  use much of it: the heaviest case in the regression net, a test fixture with a 155-layer component, uses 64.2%; the DS Base
  Demo's Card uses 23.5%.
- Swapping an instance between variants shows up as one layer becoming hidden and another visible.
- Long text changes are shown as a short excerpt (40 characters) around the first difference.
- Layer rotation is not audited or compared between variants.
- The app's Arrow is a Line with an arrowhead (the host reports it as a `LINE`), so the report calls it Line (Linha in pt-BR), not Arrow.

## Tests and regression

```sh
cargo test                      # 122 unit tests + 4 that keep English text out of the code
scripts/regress.sh              # 14 cases against tests/baseline.json: 9 in English (4 fixtures + 5 on the DS Base Demo) and 5 in pt-BR
scripts/regress.sh --private    # extra cases on files kept out of the repository (tests/private/), if present
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
