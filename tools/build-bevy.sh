#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v cargo >/dev/null && [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi
# Optional local ABI shims for systems with runtime libraries but no -dev packages.
# Normal builds use the system pkg-config metadata.
if [[ -d "$PWD/build/rust-system/lib/pkgconfig" ]]; then
  export PKG_CONFIG_PATH="$PWD/build/rust-system/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi
exec cargo build --locked --release -p sabrina -j "${SABRINA_BUILD_JOBS:-4}" "$@"
