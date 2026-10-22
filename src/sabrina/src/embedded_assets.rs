// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Serve compiled-in assets at their normal paths, without extracting files.
use bevy::{
    asset::{
        AssetApp,
        io::{
            AssetSourceBuilder, AssetSourceId,
            memory::{Dir, MemoryAssetReader},
        },
    },
    prelude::*,
};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

pub fn register(app: &mut App) {
    let root = Dir::default();
    for &(path, bytes) in FILES {
        root.insert_asset(Path::new(path), bytes);
    }
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || Box::new(MemoryAssetReader { root: root.clone() })),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_assets_and_notices_are_embedded_at_original_paths() {
        for path in [
            "models/sabrina.glb",
            "models/jellyfish.glb",
            "audio/music_calm.ogg",
            "audio/music_visit.ogg",
            "audio/jelly_appears.ogg",
            "ui/loading.jpg",
            "ui/app_icon.png",
            "ui/title_eng.png",
            "ui/title_hr.png",
            "ui/title_it.png",
            "shaders/water.wesl",
            "shaders/crystal.wesl",
            "shaders/tail.wesl",
            "fonts/LICENSE-DejaVu.txt",
            "audio/LICENSE-FluidR3.txt",
        ] {
            assert!(
                FILES
                    .iter()
                    .any(|(name, bytes)| *name == path && !bytes.is_empty()),
                "{path}"
            );
        }
        let unique: std::collections::HashSet<_> = FILES.iter().map(|(path, _)| path).collect();
        assert_eq!(unique.len(), FILES.len());
        assert!(
            !FILES
                .iter()
                .any(|(path, _)| path.starts_with("audio/help_") || *path == "ui/boot.png")
        );
    }
}
