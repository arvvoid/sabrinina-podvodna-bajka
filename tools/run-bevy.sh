#!/usr/bin/env bash
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -x target/release/sabrina ]]; then
  ./tools/build-bevy.sh
fi
exec ./target/release/sabrina "$@"
