#!/bin/sh
# Regression net: runs `generate` on the fixtures and on the DS Base Demo and compares the results with
# tests/baseline.json. `scripts/regress.sh --update [text]` rewrites the baseline from the current build
# (only the cases whose name contains `text`, when given).
# `scripts/regress.sh --private` runs only the cases in tests/private/ (a folder git ignores); without it, it just says so.
# Needs the plugin built (scripts/build.sh writes dist) and figcraft-cli on the PATH
# (or FIGCRAFT_CLI=/path/to/figcraft-cli).
set -eu
cd "$(dirname "$0")/.."
exec python3 scripts/regress.py "$@"
