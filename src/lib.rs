#![recursion_limit = "256"]
//! Handoff Components: generates the handoff documentation for a component.

mod audit;
mod contrast;
mod delta;
mod direct;
mod gerar;
mod stacked;
mod theme;
mod variants;
mod glyphs;
mod i18n;
mod panel;
mod store;

use crate::i18n::{Key, Lang, tf};
use figcraft_plugin_sdk::{Input, Result, Value, entry};

/// The manifest command: "generate" (the only one). From the menu it opens the panel, whose buttons call
/// back here with an `event`; a script passes `direct: true` to make the documentation with no panel.
fn run(input: Input) -> Result<Value> {
    match input.command.as_str() {
        "generate" if input.event.is_some() => panel::event(input),
        "generate" if direct::wanted(&input) => direct::run(input),
        "generate" => panel::open(input),
        other => Err(tf(Lang::default(), Key::ErrUnknownCommand, &[("cmd", other)])),
    }
}

entry!(run);
