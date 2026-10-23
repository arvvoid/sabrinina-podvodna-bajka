// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Bounded, nonblocking startup release check; never downloads or installs code.
use bevy::prelude::*;
use serde::Deserialize;
use std::{
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};

const API: &str = "https://api.github.com/repos/arvvoid/sabrinina-podvodna-bajka/releases/latest";
const RELEASES: &str = "https://github.com/arvvoid/sabrinina-podvodna-bajka/releases/tag/";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Checking,
    Available { tag: String, url: String },
    Continue,
}

#[derive(Resource)]
pub struct UpdateCheck {
    pub status: Status,
    pub browser_error: bool,
    receiver: Option<Mutex<mpsc::Receiver<Result<Option<(String, String)>, String>>>>,
    started: Instant,
}

impl Default for UpdateCheck {
    fn default() -> Self {
        Self {
            status: Status::Continue,
            browser_error: false,
            receiver: None,
            started: Instant::now(),
        }
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
}

fn newer_release(body: &str, current: &str) -> Result<Option<(String, String)>, String> {
    let release: Release =
        serde_json::from_str(body).map_err(|e| format!("Invalid release response: {e}"))?;
    let parse = |tag: &str| semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag));
    let latest = parse(&release.tag_name).map_err(|e| format!("Invalid release tag: {e}"))?;
    let current = parse(current).map_err(|e| format!("Invalid current version: {e}"))?;
    if release.draft
        || release.prerelease
        || !latest.pre.is_empty()
        || !latest.cmp_precedence(&current).is_gt()
    {
        return Ok(None);
    }
    // Only open a release in this repository, never an arbitrary API-provided URL.
    if !release.html_url.starts_with(RELEASES)
        || release.html_url[RELEASES.len()..].is_empty()
        || release.html_url.chars().any(char::is_whitespace)
    {
        return Err("Unexpected release page URL".into());
    }
    Ok(Some((release.tag_name, release.html_url)))
}

fn fetch(current: &str) -> Result<Option<(String, String)>, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(4)))
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(API)
        .header("User-Agent", concat!("Sabrina/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|e| format!("GitHub update check failed: {e}"))?;
    let body = response
        .body_mut()
        .with_config()
        .limit(128 * 1024)
        .read_to_string()
        .map_err(|e| format!("Could not read release response: {e}"))?;
    newer_release(&body, current)
}

pub struct UpdatePlugin;

impl Plugin for UpdatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpdateCheck>()
            .add_systems(Update, start.in_set(crate::loading::LoadResources))
            .add_systems(PreUpdate, poll);
    }
}

pub(crate) fn start(options: Res<crate::app::Options>, mut check: ResMut<UpdateCheck>) {
    if options.verify || options.capture.is_some() {
        return;
    }
    let current = crate::build_info::release_version();
    info!("Checking latest GitHub release (current version: {current})");
    let (sender, receiver) = mpsc::channel();
    match std::thread::Builder::new()
        .name("release-check".into())
        .spawn(move || {
            let _ = sender.send(fetch(current));
        }) {
        Ok(_) => {
            check.status = Status::Checking;
            check.started = Instant::now();
            check.receiver = Some(Mutex::new(receiver));
        }
        Err(error) => warn!("Could not start update check: {error}"),
    }
}

fn poll(mut check: ResMut<UpdateCheck>) {
    if check.status != Status::Checking {
        return;
    }
    let result = check.receiver.as_ref().map(|receiver| {
        receiver
            .lock()
            .expect("Update receiver poisoned")
            .try_recv()
    });
    match result {
        Some(Ok(Ok(Some((tag, url))))) => {
            info!("New release available: {tag}");
            check.status = Status::Available { tag, url };
        }
        Some(Ok(Ok(None))) => {
            info!("No newer stable release; continuing startup");
            check.status = Status::Continue;
        }
        Some(Ok(Err(error))) => {
            warn!("{error}; continuing startup");
            check.status = Status::Continue;
        }
        Some(Err(mpsc::TryRecvError::Empty))
            if check.started.elapsed() < Duration::from_secs(5) =>
        {
            return;
        }
        _ => {
            warn!("Update check timed out or disconnected; continuing startup");
            check.status = Status::Continue;
        }
    }
    check.receiver = None;
}

pub fn open_release(check: &mut UpdateCheck) -> Result<(), String> {
    open_release_with(check, webbrowser::open)
}

fn open_release_with(
    check: &mut UpdateCheck,
    open: impl FnOnce(&str) -> std::io::Result<()>,
) -> Result<(), String> {
    let Status::Available { url, .. } = &check.status else {
        return Err("No update release is available".into());
    };
    open(url).map_err(|e| format!("Could not open release page: {e}"))?;
    check.browser_error = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(tag: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"{RELEASES}{tag}","draft":false,"prerelease":false}}"#
        )
    }

    #[test]
    fn compares_semantic_versions_not_strings() {
        for (tag, current, newer) in [
            ("v0.2.10", "0.2.9", true),
            ("v0.3.0", "v0.2.6", true),
            ("v0.2.6", "0.2.6", false),
            ("v0.2.5", "0.2.6", false),
            ("v0.3.0-rc.1", "0.2.6", false),
            ("v0.2.6+build.2", "0.2.6", false),
            ("v0.2.6", "0.2.6-rc.1", true),
        ] {
            assert_eq!(newer_release(&body(tag), current).unwrap().is_some(), newer);
        }
    }

    #[test]
    fn rejects_invalid_responses_and_untrusted_urls() {
        for body in [
            "{}",
            "not json",
            &body("banana"),
            &body("v0.3.0").replace(RELEASES, "https://example.com/"),
            &body("v0.3.0").replace(&format!("{RELEASES}v0.3.0"), RELEASES),
            &body("v0.3.0").replace("tag/v0.3.0", "tag/v0.3.0 space"),
        ] {
            assert!(newer_release(body, "0.2.6").is_err());
        }
        for flag in ["draft", "prerelease"] {
            assert!(
                newer_release(
                    &body("v0.3.0")
                        .replace(&format!("\"{flag}\":false"), &format!("\"{flag}\":true")),
                    "0.2.6"
                )
                .unwrap()
                .is_none()
            );
        }
    }

    fn checking() -> (App, mpsc::Sender<Result<Option<(String, String)>, String>>) {
        let (sender, receiver) = mpsc::channel();
        let mut app = App::new();
        app.insert_resource(UpdateCheck {
            status: Status::Checking,
            receiver: Some(Mutex::new(receiver)),
            ..default()
        })
        .add_systems(PreUpdate, poll);
        (app, sender)
    }

    #[test]
    fn polling_waits_then_preserves_a_new_release_for_user_choice() {
        let (mut app, sender) = checking();
        for _ in 0..3 {
            app.update();
            assert_eq!(
                app.world().resource::<UpdateCheck>().status,
                Status::Checking
            );
        }
        sender
            .send(Ok(Some(("v0.3.0".into(), format!("{RELEASES}v0.3.0")))))
            .unwrap();
        app.update();
        assert!(matches!(
            app.world().resource::<UpdateCheck>().status,
            Status::Available { .. }
        ));
        assert!(app.world().resource::<UpdateCheck>().receiver.is_none());
        app.update();
        assert!(matches!(
            app.world().resource::<UpdateCheck>().status,
            Status::Available { .. }
        ));
    }

    #[test]
    fn no_update_network_failure_disconnection_and_timeout_all_continue() {
        for case in 0..4 {
            let (mut app, sender) = checking();
            match case {
                0 => sender.send(Ok(None)).unwrap(),
                1 => sender.send(Err("offline".into())).unwrap(),
                2 => drop(sender),
                _ => {
                    app.world_mut().resource_mut::<UpdateCheck>().started =
                        Instant::now() - Duration::from_secs(6);
                }
            }
            app.update();
            let check = app.world().resource::<UpdateCheck>();
            assert_eq!(check.status, Status::Continue);
            assert!(check.receiver.is_none());
        }
    }

    #[test]
    fn resource_loading_waits_for_splash_not_for_update_check_or_choice() {
        #[derive(Resource, Default)]
        struct Loads(usize);
        let mut app = App::new();
        app.insert_resource(UpdateCheck {
            status: Status::Checking,
            ..default()
        })
        .init_resource::<Loads>()
        .init_resource::<crate::loading::Loading>()
        .configure_sets(
            Update,
            crate::loading::LoadResources.run_if(
                crate::loading::resources_allowed
                    .and_then(bevy::ecs::schedule::common_conditions::run_once),
            ),
        )
        .add_systems(
            Update,
            (|mut loads: ResMut<Loads>| loads.0 += 1).in_set(crate::loading::LoadResources),
        );
        for _ in 0..3 {
            app.update();
            assert_eq!(app.world().resource::<Loads>().0, 0);
        }
        app.world_mut()
            .resource_mut::<crate::loading::Loading>()
            .splash_presented = true;
        for status in [
            Status::Checking,
            Status::Checking,
            Status::Available {
                tag: "v0.3.0".into(),
                url: format!("{RELEASES}v0.3.0"),
            },
        ] {
            app.world_mut().resource_mut::<UpdateCheck>().status = status;
            app.update();
            assert_eq!(app.world().resource::<Loads>().0, 1);
        }
        app.world_mut().resource_mut::<UpdateCheck>().status = Status::Continue;
        for _ in 0..3 {
            app.update();
            assert_eq!(app.world().resource::<Loads>().0, 1);
        }
    }

    #[test]
    fn no_browser_launch_without_a_release() {
        assert!(open_release(&mut UpdateCheck::default()).is_err());
    }

    #[test]
    fn browser_dispatch_preserves_prompt_on_failure_and_opens_exact_release_on_retry() {
        let mut check = UpdateCheck {
            status: Status::Available {
                tag: "v0.3.0".into(),
                url: format!("{RELEASES}v0.3.0"),
            },
            browser_error: true,
            ..default()
        };
        assert!(
            open_release_with(&mut check, |_| Err(std::io::Error::other("no browser")))
                .unwrap_err()
                .contains("no browser")
        );
        assert!(check.browser_error);
        assert!(matches!(check.status, Status::Available { .. }));
        open_release_with(&mut check, |url| {
            assert_eq!(url, format!("{RELEASES}v0.3.0"));
            Ok(())
        })
        .unwrap();
        assert!(!check.browser_error);
        assert!(matches!(check.status, Status::Available { .. }));
    }
}
