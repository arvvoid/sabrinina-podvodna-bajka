#!/usr/bin/env bash
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
exec "$(dirname "$0")/build-bevy.sh" "$@"
