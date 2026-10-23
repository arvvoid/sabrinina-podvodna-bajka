// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Cached, nonblocking release notices for standalone desktop builds.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, Write},
    path::Path,
    sync::{Mutex, mpsc},
    time::{Duration, SystemTime},
};

const API: &str = "https://api.github.com/repos/arvvoid/sabrinina-podvodna-bajka/releases/latest";
const DAY: u64 = 24 * 60 * 60;
const CHECK_INTERVAL: Duration = Duration::from_secs(30 * DAY);
const MAX_JITTER: u64 = 3 * DAY;

#[derive(Default, Deserialize, Serialize)]
struct CheckHistory {
    #[serde(default)]
    last_attempt: Option<SystemTime>,
    #[serde(default)]
    next_check: Option<SystemTime>,
    #[serde(default)]
    latest_version: Option<String>,
}

#[derive(Resource, Default)]
pub struct UpdateCheck {
    pub latest_version: Option<String>,
    receiver: Option<Mutex<mpsc::Receiver<Result<Option<String>, String>>>>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

fn version(tag: &str) -> Result<semver::Version, String> {
    semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .map_err(|e| format!("Invalid release version: {e}"))
}

fn newer_version(tag: Option<&str>, current: &str) -> Result<Option<String>, String> {
    let Some(tag) = tag else { return Ok(None) };
    let latest = version(tag)?;
    Ok(
        (latest.pre.is_empty() && latest.cmp_precedence(&version(current)?).is_gt())
            .then(|| tag.to_owned()),
    )
}

fn release_version(body: &str) -> Result<Option<String>, String> {
    let release: Release =
        serde_json::from_str(body).map_err(|e| format!("Invalid release response: {e}"))?;
    let version = version(&release.tag_name)?;
    Ok(
        (!release.draft && !release.prerelease && version.pre.is_empty())
            .then_some(release.tag_name),
    )
}

fn save_history(file: &mut File, history: &CheckHistory) -> Result<(), String> {
    let data =
        serde_json::to_vec(history).map_err(|e| format!("Cannot encode update history: {e}"))?;
    file.rewind()
        .and_then(|_| file.set_len(0))
        .and_then(|_| file.write_all(&data))
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Cannot save update history: {e}"))
}

fn check_with(
    path: &Path,
    now: SystemTime,
    current: &str,
    first_delay: Duration,
    jitter: Duration,
    request: impl FnOnce() -> Result<Option<String>, String>,
    mut publish: impl FnMut(Option<String>),
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Update history path has no directory")?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Cannot create update history directory: {e}"))?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("Cannot open update history: {e}"))?;
    // Keep concurrent launches from making duplicate requests.
    file.try_lock()
        .map_err(|e| format!("Cannot lock update history: {e}"))?;
    let mut history: CheckHistory = if file
        .metadata()
        .map_err(|e| format!("Cannot inspect update history: {e}"))?
        .len()
        == 0
    {
        CheckHistory::default()
    } else {
        serde_json::from_reader(&mut file)
            .map_err(|e| format!("Cannot read update history: {e}"))?
    };
    publish(newer_version(history.latest_version.as_deref(), current)?);
    if history.last_attempt.is_some_and(|attempt| attempt > now) {
        return Err("Last update check is in the future; deferring check".into());
    }
    let next = match history.next_check {
        Some(next) => next,
        None => {
            // Old weekly records retain their attempt time when migrating.
            let next = match history.last_attempt {
                Some(attempt) => attempt + CHECK_INTERVAL + jitter,
                None => now + first_delay,
            };
            history.next_check = Some(next);
            save_history(&mut file, &history)?;
            next
        }
    };
    if now < next
        || history
            .last_attempt
            .is_some_and(|attempt| now < attempt + CHECK_INTERVAL)
    {
        return Ok(());
    }
    history.last_attempt = Some(now);
    history.next_check = Some(now + CHECK_INTERVAL + jitter);
    // Persist failed/offline attempts too, before contacting GitHub.
    save_history(&mut file, &history)?;
    history.latest_version = request()?;
    save_history(&mut file, &history)?;
    publish(newer_version(history.latest_version.as_deref(), current)?);
    Ok(())
}

fn fetch() -> Result<Option<String>, String> {
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
    release_version(&body)
}

fn enabled(steam: bool, target: &str) -> bool {
    !steam && matches!(target, "windows" | "linux" | "macos")
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
    if !enabled(cfg!(feature = "steam"), std::env::consts::OS)
        || options.verify
        || options.capture.is_some()
    {
        return;
    }
    let history = crate::settings::Preferences::path().with_file_name("update-check.json");
    let (sender, receiver) = mpsc::channel();
    match std::thread::Builder::new()
        .name("release-check".into())
        .spawn(move || {
            let result = check_with(
                &history,
                SystemTime::now(),
                crate::build_info::release_version(),
                Duration::from_secs(fastrand::u64(DAY..=MAX_JITTER)),
                Duration::from_secs(fastrand::u64(0..=MAX_JITTER)),
                fetch,
                |version| {
                    let _ = sender.send(Ok(version));
                },
            );
            if let Err(error) = result {
                let _ = sender.send(Err(error));
            }
        }) {
        Ok(_) => check.receiver = Some(Mutex::new(receiver)),
        Err(error) => warn!("Could not start update check: {error}"),
    }
}

fn poll(mut check: ResMut<UpdateCheck>) {
    loop {
        let Some(receiver) = &check.receiver else {
            return;
        };
        let result = receiver
            .lock()
            .expect("Update receiver poisoned")
            .try_recv();
        match result {
            Ok(Ok(version)) => check.latest_version = version,
            Ok(Err(error)) => warn!("{error}; update notice unchanged"),
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                check.receiver = None;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "sabrina-update-{name}-{}-{stamp}",
                std::process::id()
            ))
            .join("update-check.json")
    }

    fn read(path: &Path) -> CheckHistory {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    fn remove_history(path: &Path) {
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    fn check(
        path: &Path,
        now: SystemTime,
        current: &str,
        request: impl FnOnce() -> Result<Option<String>, String>,
    ) -> Result<Vec<Option<String>>, String> {
        let mut notices = Vec::new();
        check_with(
            path,
            now,
            current,
            Duration::from_secs(2 * DAY),
            Duration::from_secs(DAY),
            request,
            |version| notices.push(version),
        )?;
        Ok(notices)
    }

    #[test]
    fn first_check_is_deferred_and_deadline_survives_restarts() {
        for days in [1, 2, 3] {
            let path = history_path("first");
            let now = SystemTime::now();
            let delay = Duration::from_secs(days * DAY);
            check_with(
                &path,
                now,
                "0.2.6",
                delay,
                Duration::ZERO,
                || panic!("First launch must not contact GitHub"),
                |_| {},
            )
            .unwrap();
            let next = now + delay;
            assert_eq!(read(&path).next_check, Some(next));
            assert_eq!(read(&path).last_attempt, None);
            check(&path, next - Duration::from_nanos(1), "0.2.6", || {
                panic!("Restart must not reset the deadline")
            })
            .unwrap();
            assert_eq!(read(&path).next_check, Some(next));
            check(&path, next, "0.2.6", || Ok(Some("v0.3.0".into()))).unwrap();
            assert_eq!(read(&path).last_attempt, Some(next));
            remove_history(&path);
        }
    }

    #[test]
    fn successful_check_is_cached_until_randomized_monthly_deadline() {
        let path = history_path("cached");
        let now = SystemTime::now();
        check(&path, now, "0.2.6", || panic!()).unwrap();
        let attempt = read(&path).next_check.unwrap();
        assert_eq!(
            check(&path, attempt, "0.2.6", || {
                assert_eq!(read(&path).last_attempt, Some(attempt));
                Ok(Some("v0.3.0".into()))
            })
            .unwrap(),
            [None, Some("v0.3.0".into())]
        );
        let next = attempt + CHECK_INTERVAL + Duration::from_secs(DAY);
        assert_eq!(read(&path).next_check, Some(next));
        for elapsed in [
            Duration::ZERO,
            CHECK_INTERVAL,
            next.duration_since(attempt).unwrap() - Duration::from_nanos(1),
        ] {
            assert_eq!(
                check(&path, attempt + elapsed, "0.2.6", || panic!("Too early")).unwrap(),
                [Some("v0.3.0".into())]
            );
        }
        assert_eq!(
            check(&path, next - Duration::from_secs(1), "0.3.0", || panic!()).unwrap(),
            [None]
        );
        assert_eq!(
            check(&path, next, "0.2.6", || Ok(Some("v0.4.0".into()))).unwrap(),
            [Some("v0.3.0".into()), Some("v0.4.0".into())]
        );
        remove_history(&path);
    }

    #[test]
    fn failed_attempt_preserves_cache_and_schedules_next_month() {
        let path = history_path("offline");
        let now = SystemTime::now();
        check(&path, now, "0.2.6", || panic!()).unwrap();
        let first = read(&path).next_check.unwrap();
        check(&path, first, "0.2.6", || Ok(Some("v0.3.0".into()))).unwrap();
        let attempt = read(&path).next_check.unwrap();
        assert_eq!(
            check(&path, attempt, "0.2.6", || Err("offline".into())).unwrap_err(),
            "offline"
        );
        let history = read(&path);
        assert_eq!(history.latest_version.as_deref(), Some("v0.3.0"));
        assert_eq!(history.last_attempt, Some(attempt));
        assert_eq!(
            history.next_check,
            Some(attempt + CHECK_INTERVAL + Duration::from_secs(DAY))
        );
        assert_eq!(
            check(&path, attempt + CHECK_INTERVAL, "0.2.6", || panic!(
                "Early retry"
            ))
            .unwrap(),
            [Some("v0.3.0".into())]
        );
        remove_history(&path);
    }

    #[test]
    fn weekly_records_migrate_without_resetting_last_attempt() {
        for jitter in [Duration::ZERO, Duration::from_secs(MAX_JITTER)] {
            let path = history_path("migration");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let now = SystemTime::now();
            std::fs::write(
                &path,
                serde_json::to_vec(&serde_json::json!({"last_attempt": now})).unwrap(),
            )
            .unwrap();
            check_with(
                &path,
                now + Duration::from_secs(7 * DAY),
                "0.2.6",
                Duration::from_secs(DAY),
                jitter,
                || panic!("Weekly check must be deferred"),
                |_| {},
            )
            .unwrap();
            assert_eq!(read(&path).next_check, Some(now + CHECK_INTERVAL + jitter));
            assert_eq!(read(&path).last_attempt, Some(now));
            remove_history(&path);
        }
    }

    #[test]
    fn persistence_errors_and_clock_rollback_never_make_requests() {
        let path = history_path("invalid");
        let now = SystemTime::now();
        check(&path, now, "0.2.6", || panic!()).unwrap();
        let attempt = read(&path).next_check.unwrap();
        check(&path, attempt, "0.2.6", || Ok(None)).unwrap();
        assert!(
            check(&path, now, "0.2.6", || panic!())
                .unwrap_err()
                .contains("future")
        );
        std::fs::write(&path, "not json").unwrap();
        assert!(
            check(&path, now, "0.2.6", || panic!())
                .unwrap_err()
                .contains("Cannot read")
        );
        remove_history(&path);
        let path = history_path("blocked");
        std::fs::write(path.parent().unwrap(), "not a directory").unwrap();
        assert!(check(&path, now, "0.2.6", || panic!()).is_err());
        std::fs::remove_file(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn simultaneous_launches_make_only_one_request() {
        use std::sync::{
            Arc, Barrier,
            atomic::{AtomicUsize, Ordering},
        };
        let path = history_path("concurrent");
        check(&path, SystemTime::now(), "0.2.6", || panic!()).unwrap();
        let now = read(&path).next_check.unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let requests = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let barrier = barrier.clone();
                let path = &path;
                let requests = &requests;
                scope.spawn(move || {
                    barrier.wait();
                    if let Err(error) = check(path, now, "0.2.6", || {
                        requests.fetch_add(1, Ordering::SeqCst);
                        Ok(None)
                    }) {
                        assert!(error.contains("Cannot lock"), "{error}");
                    }
                });
            }
        });
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        remove_history(&path);
    }

    #[test]
    fn only_standalone_desktop_builds_enable_checks() {
        for target in ["windows", "linux", "macos"] {
            assert!(enabled(false, target));
            assert!(!enabled(true, target));
        }
        for target in ["android", "ios", "unknown"] {
            assert!(!enabled(false, target));
            assert!(!enabled(true, target));
        }
        if cfg!(feature = "steam") {
            use bevy::ecs::system::RunSystemOnce;
            let mut app = App::new();
            app.init_resource::<UpdateCheck>()
                .init_resource::<crate::app::Options>();
            app.world_mut().run_system_once(start).unwrap();
            assert!(app.world().resource::<UpdateCheck>().receiver.is_none());
        }
    }

    #[test]
    fn cached_versions_are_compared_semantically_and_hidden_after_upgrading() {
        for (tag, current, newer) in [
            ("v0.2.10", "0.2.9", true),
            ("v0.3.0", "v0.2.6", true),
            ("v0.2.6", "0.2.6", false),
            ("v0.2.5", "0.2.6", false),
            ("v0.3.0-rc.1", "0.2.6", false),
            ("v0.2.6+build.2", "0.2.6", false),
        ] {
            assert_eq!(newer_version(Some(tag), current).unwrap().is_some(), newer);
        }
        assert!(newer_version(Some("invalid"), "0.2.6").is_err());
        assert!(release_version("{}").is_err());
        for flag in ["draft", "prerelease"] {
            let body = r#"{"tag_name":"v0.3.0","draft":false,"prerelease":false}"#
                .replace(&format!("\"{flag}\":false"), &format!("\"{flag}\":true"));
            assert_eq!(release_version(&body).unwrap(), None);
        }
    }

    #[test]
    fn polling_keeps_cached_notice_during_pending_requests_and_failures() {
        let (sender, receiver) = mpsc::channel();
        let mut app = App::new();
        app.insert_resource(UpdateCheck {
            receiver: Some(Mutex::new(receiver)),
            ..default()
        })
        .add_systems(Update, poll);
        sender.send(Ok(Some("v0.3.0".into()))).unwrap();
        app.update();
        for _ in 0..3 {
            app.update();
            assert_eq!(
                app.world()
                    .resource::<UpdateCheck>()
                    .latest_version
                    .as_deref(),
                Some("v0.3.0")
            );
        }
        sender.send(Err("offline".into())).unwrap();
        drop(sender);
        app.update();
        let check = app.world().resource::<UpdateCheck>();
        assert_eq!(check.latest_version.as_deref(), Some("v0.3.0"));
        assert!(check.receiver.is_none());
    }
}
