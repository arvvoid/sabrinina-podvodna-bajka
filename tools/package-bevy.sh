#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
cd "$(dirname "$0")/.."
./tools/build-bevy.sh
package="$PWD/dist/sabrina-bevy-$(uname -s)-$(uname -m)"
mkdir -p "$package"
# Replace previous package contents so removed assets cannot linger.
rm -rf "$package/assets" "$package/docs"
cp target/release/sabrina "$package/"
# Ship runtime assets; SVG originals stay in the source tree.
python3 - "$package" <<'PY'
import shutil, sys
from pathlib import Path
root = Path('assets')
target = Path(sys.argv[1])
for folder in ['models', 'render_lods', 'environments', 'props', 'audio', 'fonts', 'credits', 'ui/reef_png', 'shaders']:
    for source in (root / folder).rglob('*'):
        if source.is_file() and source.suffix not in {'.import', '.uid', '.md'}:
            dest = target / 'assets' / source.relative_to(root)
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, dest)
shutil.copy2(root / 'ui/boot.png', target / 'assets/ui/boot.png')
shutil.copytree('docs', target / 'docs', dirs_exist_ok=True)
for name in ['LICENSE', 'ASSET_LICENSE.md', 'README.md']:
    shutil.copy2(name, target / name)
PY
printf 'Portable game directory: %s\n' "$package"
