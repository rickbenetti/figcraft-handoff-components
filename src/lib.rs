#![recursion_limit = "256"]
//! Handoff Components: generates the handoff documentation for a component.

// The 0.10.0 drawing code (anatomy, spacing, axis tables, audit and the old one-frame `generate`) is kept for parts B and C of
// 0.11 and is not called yet: these modules, and only these, may have dead code. `gerar::section` (the 0.11 document) is checked.
#[allow(dead_code)]
mod audit;
mod contrast;
#[allow(dead_code)]
mod delta;
mod direct;
#[allow(dead_code)]
mod gerar;
#[allow(dead_code)]
mod glyphs;
mod i18n;
mod panel;
#[allow(dead_code)]
mod stacked;
mod store;
#[allow(dead_code)]
mod theme;
#[allow(dead_code)]
mod variants;

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
