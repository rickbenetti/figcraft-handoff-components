#![recursion_limit = "256"]
//! Handoff Components: generates the handoff report for a component.

mod audit;
mod contrast;
mod delta;
mod gerar;
mod theme;
mod variants;
mod glyphs;
mod i18n;
mod panel;
mod settings;
mod store;

use crate::i18n::{Key, Lang, tf};
use figcraft_plugin_sdk::{Input, Result, Value, entry};

/// The manifest command picked: "generate" makes the component report, "settings" saves the settings.
fn run(input: Input) -> Result<Value> {
    match input.command.as_str() {
        "generate" => gerar::generate(input),
        "settings" => settings::settings(input),
        other => Err(tf(Lang::default(), Key::ErrUnknownCommand, &[("cmd", other)])),
    }
}

entry!(run);
