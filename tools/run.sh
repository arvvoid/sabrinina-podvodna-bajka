#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
cd "$(dirname "$0")/.."
engine="${GODOT_BIN:-$PWD/tools/godot}"
if [[ ! -x "$engine" ]]; then
  echo 'Godot not found. Run ./tools/bootstrap.sh, or set GODOT_BIN to your Godot executable.' >&2
  exit 1
fi
if [[ ! -f game/bin/libsabrina.so && ! -f game/bin/libsabrina.dylib && ! -f game/bin/sabrina.dll ]]; then
  echo 'Native game library not found. Run ./tools/build.sh first.' >&2
  exit 1
fi
if [[ ! -f game/.godot/extension_list.cfg ]]; then
  # Godot cold-import documentation cleanup race: godotengine/godot#111645.
  # Give deferred extension documentation work time to finish before import exits.
  "$engine" --headless --editor --path game --import --frame-delay 800
fi
exec "$engine" --path game "$@"
