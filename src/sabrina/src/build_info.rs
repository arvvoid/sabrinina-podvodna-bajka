// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Build identity embedded at compile time.

const VERSION: &str = env!("SABRINA_BUILD_VERSION");
const HASH: &str = env!("SABRINA_BUILD_HASH");
const DATE: &str = env!("SABRINA_BUILD_DATE");

pub fn menu_label() -> String {
    format!("{VERSION} · {HASH} · {DATE}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_identity_includes_version_hash_and_build_date() {
        assert_eq!(menu_label().split(" · ").count(), 3);
        assert!(!VERSION.is_empty());
        assert!(
            HASH == "unknown"
                || (HASH.len() == 12 && HASH.bytes().all(|byte| byte.is_ascii_hexdigit()))
        );
        assert_eq!(DATE.len(), 10);
        assert_eq!(&DATE[4..5], "-");
        assert_eq!(&DATE[7..8], "-");
        assert_eq!(menu_label(), format!("{VERSION} · {HASH} · {DATE}"));
    }
}
