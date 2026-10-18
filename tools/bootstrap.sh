#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
cd "$(dirname "$0")/.."
git submodule update --init --recursive
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo 'Install Godot 4.7.2 for your platform from https://godotengine.org/download/ and use it with game/.'
  exit 0
fi
if [[ ! -x tools/godot ]]; then
  archive=$(mktemp /tmp/sabrina-godot.XXXXXX.zip)
  trap 'rm -f "$archive"' EXIT
  curl -fL --retry 3 'https://github.com/godotengine/godot/releases/download/4.7.2-stable/Godot_v4.7.2-stable_linux.x86_64.zip' -o "$archive"
  echo "cadd3204e728a35d3f13adb7fd0d7902636b79f6b95c40c265eb73b6c35329e4  $archive" | sha256sum -c -
  unzip -o "$archive" -d tools
  mv tools/Godot_v4.7.2-stable_linux.x86_64 tools/godot
  chmod +x tools/godot
fi
./tools/godot --version
