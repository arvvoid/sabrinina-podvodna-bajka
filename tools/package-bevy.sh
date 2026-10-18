#!/usr/bin/env bash
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: MIT
set -euo pipefail
cd "$(dirname "$0")/.."
./tools/build-bevy.sh
package="$PWD/dist/sabrina-bevy-$(uname -s)-$(uname -m)"
python3 tools/package_release.py --binary target/release/sabrina \
  --name "$(basename "$package")" --format directory
printf 'Portable game directory: %s\n' "$package"
