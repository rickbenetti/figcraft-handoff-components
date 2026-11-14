#!/usr/bin/env bash
# Builds the plugin and packs the release zip: dist/handoff-components-<version>.zip
# The zip holds exactly four files at its root (no dist/ folder):
#   figcraft-plugin.json, handoff_components.wasm, LICENSE, THIRD_PARTY_NOTICES.md
#
# Reproducible: the files are staged in a temporary folder outside the repository, in a fixed order,
# with mode 644 (755 for the .wasm, as in the 0.9.6 zip) and a fixed modification time (2026-01-01 00:00:00 UTC, set with TZ=UTC touch -t), and the zip
# is made with `zip -X -9` (no extra file attributes). The same inputs give the same zip, byte for byte.
#
# It does not tag, publish or upload anything.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
FIXED_TIME="202601010000"   # touch -t, read as UTC

die() { echo "package.sh: $*" >&2; exit 1; }

command -v zip >/dev/null || die "zip not found on the PATH"
command -v unzip >/dev/null || die "unzip not found on the PATH"
command -v shasum >/dev/null || die "shasum not found on the PATH"

# 1. Build (writes dist/figcraft-plugin.json and dist/handoff_components.wasm).
"$ROOT/scripts/build.sh"

# 2. The four files must exist.
MANIFEST="$DIST/figcraft-plugin.json"
WASM="$DIST/handoff_components.wasm"
LICENSE_FILE="$ROOT/LICENSE"
NOTICES="$ROOT/THIRD_PARTY_NOTICES.md"
[ -f "$WASM" ] || die "missing $WASM (the build did not write it)"
[ -f "$MANIFEST" ] || die "missing $MANIFEST"
[ -f "$LICENSE_FILE" ] || die "missing $LICENSE_FILE"
[ -f "$NOTICES" ] || die "missing $NOTICES"

# 3. Version: from the manifest (python3 when present, grep/sed otherwise), checked against Cargo.toml.
if command -v python3 >/dev/null; then
  VERSION="$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$MANIFEST")"
else
  VERSION="$(grep -m1 '"version"' "$MANIFEST" | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"([^"]*)".*/\1/')"
fi
echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || die "the manifest version '$VERSION' is not X.Y.Z"
CARGO_VERSION="$(grep -m1 -E '^version[[:space:]]*=' "$ROOT/Cargo.toml" | sed -E 's/^version[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/')"
[ "$VERSION" = "$CARGO_VERSION" ] || die "the manifest version ($VERSION) differs from Cargo.toml ($CARGO_VERSION)"

# 4. Stage the four files in a temporary folder outside the repository.
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/handoff-package.XXXXXX")"
trap '/bin/rm -rf "$STAGE"' EXIT
/bin/cp "$MANIFEST" "$STAGE/figcraft-plugin.json"
/bin/cp "$WASM" "$STAGE/handoff_components.wasm"
/bin/cp "$LICENSE_FILE" "$STAGE/LICENSE"
/bin/cp "$NOTICES" "$STAGE/THIRD_PARTY_NOTICES.md"
chmod 644 "$STAGE"/*
chmod 755 "$STAGE/handoff_components.wasm"   # as in the 0.9.6 zip
TZ=UTC touch -t "$FIXED_TIME" "$STAGE"/*

# 5. Zip, in a fixed order. A previous zip with the same name is replaced (zip would otherwise update it in place).
ZIP="$DIST/handoff-components-$VERSION.zip"
/bin/rm -f "$ZIP"
(cd "$STAGE" && TZ=UTC zip -X -9 -q "$ZIP" figcraft-plugin.json handoff_components.wasm LICENSE THIRD_PARTY_NOTICES.md)

# 6. Report.
echo
echo "Package: $ZIP"
echo "Version: $VERSION"
echo "Zip size: $(wc -c < "$ZIP" | tr -d ' ') bytes"
echo "Zip SHA-256:  $(shasum -a 256 "$ZIP" | cut -d' ' -f1)"
echo "Wasm SHA-256: $(shasum -a 256 "$WASM" | cut -d' ' -f1)"
echo
unzip -l "$ZIP"
