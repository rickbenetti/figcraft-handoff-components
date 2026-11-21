#!/bin/sh
# Builds the plugin (wasm32, release) and writes the importable folder: dist/
# (figcraft-plugin.json + handoff_components.wasm). That folder is the only place to import from.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
cd "$ROOT"
# The .wasm is published: rustc embeds the place of every `panic!` of the crates it builds (and of the standard library), so no path of this
# machine may reach it. Each prefix is rewritten to a neutral one. RUSTFLAGS that the caller already set are kept and these are added after
# them (note that a RUSTFLAGS in the environment takes the place of `build.rustflags` in a cargo config, as cargo always does).
CARGO_DIR="${CARGO_HOME:-$HOME/.cargo}"
RUSTUP_DIR="${RUSTUP_HOME:-$HOME/.rustup}"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--remap-path-prefix=$CARGO_DIR=/cargo --remap-path-prefix=$RUSTUP_DIR=/rustup --remap-path-prefix=$ROOT=/build"
export RUSTFLAGS
cargo build --release --target wasm32-unknown-unknown
mkdir -p "$DIST"
/bin/cp -f "$ROOT/figcraft-plugin.json" "$DIST/figcraft-plugin.json"
/bin/cp -f "$ROOT/target/wasm32-unknown-unknown/release/handoff_components.wasm" "$DIST/handoff_components.wasm"
echo "Import this manifest: $DIST/figcraft-plugin.json"
wc -c "$DIST/figcraft-plugin.json" "$DIST/handoff_components.wasm"
