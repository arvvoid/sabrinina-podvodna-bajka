// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn git_output(root: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}

// Gregorian civil date from days since 1970-01-01, in UTC.
fn utc_date(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted / 146_097
    } else {
        (shifted - 146_096) / 146_097
    };
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn version(actions: bool, ref_type: Option<&str>, ref_name: Option<&str>) -> String {
    if actions && ref_type == Some("tag") {
        let tag = ref_name
            .filter(|name| !name.is_empty())
            .expect("GitHub tag builds require GITHUB_REF_NAME");
        assert!(!tag.contains(['\r', '\n']), "Invalid build tag");
        tag.to_owned()
    } else {
        "DEV".to_owned()
    }
}

fn short_hash(hash: &str) -> String {
    assert!(
        matches!(hash.len(), 40 | 64) && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Build commit must be a full Git SHA"
    );
    hash[..12].to_owned()
}

fn watch_git_path(root: &Path, name: &str) {
    if let Some(path) = git_output(root, &["rev-parse", "--git-path", name]) {
        let path = PathBuf::from(path);
        let mut path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        // Packed refs and new loose refs may not exist yet. Watch their nearest
        // existing parent so packing/unpacking or creating a ref invalidates us.
        while !path.exists() {
            if !path.pop() {
                return;
            }
        }
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn embedded_assets(root: &Path) {
    fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(directory).expect("Cannot read asset directory") {
            let entry = entry.expect("Cannot read asset entry");
            let kind = entry.file_type().expect("Cannot inspect asset entry");
            assert!(
                !kind.is_symlink(),
                "Embedded assets must not be symlinks: {:?}",
                entry.path()
            );
            if kind.is_dir() {
                collect(&entry.path(), files);
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    let assets = root
        .join("../../assets")
        .canonicalize()
        .expect("Missing runtime assets");
    println!("cargo:rerun-if-changed={}", assets.display());
    let mut files = Vec::new();
    collect(&assets, &mut files);
    files.sort();
    assert!(!files.is_empty(), "Cannot build without runtime assets");
    let mut source = String::from("pub const FILES: &[(&str, &[u8])] = &[\n");
    for file in files {
        let relative = file
            .strip_prefix(&assets)
            .expect("Asset outside root")
            .to_str()
            .expect("Asset path must be UTF-8")
            .replace('\\', "/");
        source.push_str(&format!(
            "({relative:?}, include_bytes!({:?})),\n",
            file.to_str().expect("Asset path must be UTF-8")
        ));
    }
    source.push_str("];\n");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    std::fs::write(output.join("embedded_assets.rs"), source)
        .expect("Cannot generate embedded asset table");
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets manifest dir"));
    embedded_assets(&root);
    for source in [
        "build.rs",
        "src",
        "Cargo.toml",
        "../reef_sim",
        "../../Cargo.toml",
        "../../Cargo.lock",
        "../../rust-toolchain.toml",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(source).display());
    }
    watch_git_path(&root, "HEAD");
    watch_git_path(&root, "packed-refs");
    if let Some(reference) = git_output(&root, &["symbolic-ref", "-q", "HEAD"]) {
        watch_git_path(&root, &reference);
    }
    for variable in [
        "GITHUB_ACTIONS",
        "GITHUB_REF_TYPE",
        "GITHUB_REF_NAME",
        "GITHUB_SHA",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let actions = env::var("GITHUB_ACTIONS").as_deref() == Ok("true");
    let version = version(
        actions,
        env::var("GITHUB_REF_TYPE").ok().as_deref(),
        env::var("GITHUB_REF_NAME").ok().as_deref(),
    );
    let hash = git_output(&root, &["rev-parse", "--verify", "HEAD"])
        .or_else(|| {
            println!("cargo:warning=Git commit unavailable in source checkout.");
            actions.then(|| env::var("GITHUB_SHA").ok()).flatten()
        })
        .map(|hash| short_hash(&hash))
        .unwrap_or_else(|| {
            assert!(version == "DEV", "Release builds require a commit SHA");
            "unknown".to_owned()
        });
    let date = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Build time must be after the Unix epoch")
        .as_secs();
    println!("cargo:rustc-env=SABRINA_BUILD_VERSION={version}");
    println!("cargo:rustc-env=SABRINA_BUILD_HASH={hash}");
    println!("cargo:rustc-env=SABRINA_BUILD_DATE={}", utc_date(date));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_github_tag_builds_use_a_release_version() {
        assert_eq!(version(false, Some("tag"), Some("v1.2.3")), "DEV");
        assert_eq!(version(true, Some("branch"), Some("main")), "DEV");
        assert_eq!(
            version(true, Some("tag"), Some("v1.2.3-beta.1")),
            "v1.2.3-beta.1"
        );
        assert_eq!(version(false, None, None), "DEV");
    }

    #[test]
    #[should_panic(expected = "GITHUB_REF_NAME")]
    fn missing_release_tag_is_an_error() {
        version(true, Some("tag"), None);
    }

    #[test]
    fn commit_is_exactly_twelve_hex_characters() {
        assert_eq!(
            short_hash("0123456789abcdef0123456789abcdef01234567"),
            "0123456789ab"
        );
    }

    #[test]
    #[should_panic(expected = "full Git SHA")]
    fn malformed_sha_is_an_error() {
        short_hash("unknown000000");
    }

    #[test]
    fn utc_dates_cover_epoch_leap_year_and_year_boundary() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(951_782_400), "2000-02-29");
        assert_eq!(utc_date(1_704_067_199), "2023-12-31");
        assert_eq!(utc_date(1_704_067_200), "2024-01-01");
        assert_eq!(utc_date(1_792_281_600), "2026-10-18");
    }
}
