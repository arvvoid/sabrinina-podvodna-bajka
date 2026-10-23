#!/usr/bin/env bash
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
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
if command -v cargo-about >/dev/null; then
  cargo about generate --locked --all-features --fail about.hbs -o THIRD_PARTY_LICENSES.txt
else
  echo "warning: cargo-about is unavailable; using existing dependency notices, if present." >&2
  echo "Install with: cargo install cargo-about --locked --version 0.9.2 --features cli" >&2
fi
exec cargo build --locked --release -p sabrina -j "${SABRINA_BUILD_JOBS:-4}" "$@"
