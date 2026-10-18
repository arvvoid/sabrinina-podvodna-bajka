#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -x target/release/sabrina ]]; then
  ./tools/build-bevy.sh
fi
exec ./target/release/sabrina "$@"
