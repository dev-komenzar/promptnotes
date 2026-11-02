//! Tests for slice `install-update`.
//!
//! Spec: `.ori/slices/install-update/spec.md#test-points`。
//! `UpdateInstaller` は adapter 境界の test double (`FakeInstaller`)。

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use proptest::prelude::*;
use tauri::utils::config::BundleType;

use crate::update_distribution::shared::ports::{InstallGuard, PendingUpdate, UpdateInstaller};
use crate::update_distribution::shared::types::{
    BundleFormat, InstallUpdateError, InstallationKind, Version,
};

use super::commands::InstallUpdateErrorResponse;
use super::domain::InstallPhase;
use super::infrastructure::{installation_kind_from, map_download_error, InMemoryInstallState};
use super::{InstallUpdateCommand, InstallUpdateUseCase};

#[derive(Clone, Copy)]
enum InstallOutcome {
    Ok,
    DownloadFails,
    InstallFails,
}

struct FakeInstaller {
    fetch: Result<String, InstallUpdateError>,
    outcome: InstallOutcome,
    fetch_calls: Rc<Cell<u32>>,
    install_calls: Rc<Cell<u32>>,
}

impl FakeInstaller {
    fn new(fetch: Result<&str, InstallUpdateError>, outcome: InstallOutcome) -> Self {
        Self {
            fetch: fetch.map(str::to_string),
            outcome,
            fetch_calls: Rc::new(Cell::new(0)),
            install_calls: Rc::new(Cell::new(0)),
        }
    }
}

struct FakePending {
    version: String,
    outcome: InstallOutcome,
    install_calls: Rc<Cell<u32>>,
}

impl PendingUpdate for FakePending {
    fn version_string(&self) -> &str {
        &self.version
    }
    fn download_and_install(self: Box<Self>) -> Result<(), InstallUpdateError> {
        self.install_calls.set(self.install_calls.get() + 1);
        match self.outcome {
            InstallOutcome::Ok => Ok(()),
            InstallOutcome::DownloadFails => Err(InstallUpdateError::DownloadFailed("net".into())),
            InstallOutcome::InstallFails => Err(InstallUpdateError::InstallFailed("sig".into())),
        }
    }
}

impl UpdateInstaller for FakeInstaller {
    fn fetch_pending(&self) -> Result<Box<dyn PendingUpdate>, InstallUpdateError> {
        self.fetch_calls.set(self.fetch_calls.get() + 1);
        let version = self.fetch.clone()?;
        Ok(Box::new(FakePending {
            version,
            outcome: self.outcome,
            install_calls: self.install_calls.clone(),
        }))
    }
}

fn v(s: &str) -> Version {
    Version::from_str(s).expect("test version")
}

fn cmd(current: &str, installation: InstallationKind) -> InstallUpdateCommand {
    InstallUpdateCommand {
        current_version: v(current),
        installation,
    }
}

const APPIMAGE: InstallationKind = InstallationKind::Bundled(BundleFormat::AppImage);

/// (result, fetch 回数, install 回数)。実行状態は毎回新しい production state (Idle) を使う。
fn run(
    current: &str,
    installation: InstallationKind,
    fetch: Result<&str, InstallUpdateError>,
    outcome: InstallOutcome,
) -> (Result<Version, InstallUpdateError>, u32, u32) {
    run_with(
        &Arc::new(InMemoryInstallState::new()),
        current,
        installation,
        fetch,
        outcome,
    )
}

/// 共有の実行状態 (production の `InMemoryInstallState`) で 1 回 install する。
fn run_with(
    state: &Arc<InMemoryInstallState>,
    current: &str,
    installation: InstallationKind,
    fetch: Result<&str, InstallUpdateError>,
    outcome: InstallOutcome,
) -> (Result<Version, InstallUpdateError>, u32, u32) {
    let installer = FakeInstaller::new(fetch, outcome);
    let (fetches, installs) = (
        installer.fetch_calls.clone(),
        installer.install_calls.clone(),
    );
    let result =
        InstallUpdateUseCase::new(installer, state.clone()).execute(cmd(current, installation));
    (result, fetches.get(), installs.get())
}

// ===== spec.md#tp-install-success =====

#[test]
fn tp_s1_installs_newer_release_and_returns_its_version() {
    let (result, fetches, installs) = run("0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::Ok);
    assert_eq!(result, Ok(v("0.2.5")));
    assert_eq!((fetches, installs), (1, 1));
}

#[test]
fn tp_s2_installs_the_refetched_latest_even_if_newer_than_notified() {
    let (result, _, installs) = run("0.2.4", APPIMAGE, Ok("0.2.6"), InstallOutcome::Ok);
    assert_eq!(result, Ok(v("0.2.6")));
    assert_eq!(installs, 1);
}

// ===== spec.md#tp-unbundled =====

#[test]
fn tp_u1_unbundled_is_rejected_without_touching_updater() {
    let (result, fetches, installs) = run(
        "0.2.4",
        InstallationKind::Unbundled,
        Ok("0.2.5"),
        InstallOutcome::Ok,
    );
    assert_eq!(result, Err(InstallUpdateError::UnsupportedInstallation));
    assert_eq!((fetches, installs), (0, 0));
}

#[test]
fn tp_u2_only_bundled_is_installable() {
    for f in [
        BundleFormat::MacApp,
        BundleFormat::AppImage,
        BundleFormat::Deb,
        BundleFormat::Rpm,
        BundleFormat::Msi,
        BundleFormat::Nsis,
    ] {
        assert!(InstallationKind::Bundled(f).is_installable());
    }
    assert!(!InstallationKind::Unbundled.is_installable());
}

#[test]
fn tp_u3_bundle_type_maps_to_installation_kind() {
    use BundleFormat::*;
    let cases = [
        (Some(BundleType::App), InstallationKind::Bundled(MacApp)),
        (Some(BundleType::Dmg), InstallationKind::Bundled(MacApp)),
        (
            Some(BundleType::AppImage),
            InstallationKind::Bundled(AppImage),
        ),
        (Some(BundleType::Deb), InstallationKind::Bundled(Deb)),
        (Some(BundleType::Rpm), InstallationKind::Bundled(Rpm)),
        (Some(BundleType::Msi), InstallationKind::Bundled(Msi)),
        (Some(BundleType::Nsis), InstallationKind::Bundled(Nsis)),
        (None, InstallationKind::Unbundled),
    ];
    for (bundle, expected) in cases {
        assert_eq!(installation_kind_from(bundle), expected);
    }
}

// ===== spec.md#tp-not-newer =====

#[test]
fn tp_n1_same_or_older_release_is_not_installed() {
    for fetched in ["0.2.4", "0.2.3"] {
        let (result, _, installs) = run("0.2.4", APPIMAGE, Ok(fetched), InstallOutcome::Ok);
        assert_eq!(
            result,
            Err(InstallUpdateError::NoUpdateAvailable),
            "{fetched}"
        );
        assert_eq!(installs, 0, "{fetched}");
    }
}

proptest! {
    /// TP-N2: install が呼ばれるのは fetched > current のときだけ。
    #[test]
    fn tp_n2_installs_iff_fetched_is_newer(
        c in (0u32..5, 0u32..5, 0u32..5),
        f in (0u32..5, 0u32..5, 0u32..5),
    ) {
        let current = format!("{}.{}.{}", c.0, c.1, c.2);
        let fetched = format!("{}.{}.{}", f.0, f.1, f.2);
        let (result, _, installs) = run(&current, APPIMAGE, Ok(&fetched), InstallOutcome::Ok);
        let newer = v(&fetched) > v(&current);
        prop_assert_eq!(installs == 1, newer);
        prop_assert_eq!(result.is_ok(), newer);
    }
}

// ===== spec.md#tp-failures =====

#[test]
fn tp_f1_fetch_failure_is_check_failed_and_skips_install() {
    let (result, fetches, installs) = run(
        "0.2.4",
        APPIMAGE,
        Err(InstallUpdateError::CheckFailed("offline".into())),
        InstallOutcome::Ok,
    );
    assert_eq!(
        result,
        Err(InstallUpdateError::CheckFailed("offline".into()))
    );
    assert_eq!((fetches, installs), (1, 0));
}

#[test]
fn tp_f2_unparseable_version_is_check_failed() {
    let (result, _, installs) = run("0.2.4", APPIMAGE, Ok("not-a-version"), InstallOutcome::Ok);
    assert!(matches!(result, Err(InstallUpdateError::CheckFailed(_))));
    assert_eq!(installs, 0);
}

#[test]
fn tp_f3_download_and_install_failures_are_propagated() {
    let (result, fetches, installs) = run(
        "0.2.4",
        APPIMAGE,
        Ok("0.2.5"),
        InstallOutcome::DownloadFails,
    );
    assert!(matches!(result, Err(InstallUpdateError::DownloadFailed(_))));
    assert_eq!((fetches, installs), (1, 1));

    let (result, fetches, installs) =
        run("0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::InstallFails);
    assert!(matches!(result, Err(InstallUpdateError::InstallFailed(_))));
    assert_eq!((fetches, installs), (1, 1));
}

// ===== spec.md#tp-boundary =====

#[test]
fn tp_b3_error_response_kind_matches_variant_name() {
    let cases = [
        (
            InstallUpdateError::UnsupportedInstallation,
            "UnsupportedInstallation",
        ),
        (InstallUpdateError::NoUpdateAvailable, "NoUpdateAvailable"),
        (InstallUpdateError::InstallInProgress, "InstallInProgress"),
        (InstallUpdateError::CheckFailed("x".into()), "CheckFailed"),
        (
            InstallUpdateError::DownloadFailed("x".into()),
            "DownloadFailed",
        ),
        (
            InstallUpdateError::InstallFailed("x".into()),
            "InstallFailed",
        ),
    ];
    for (err, kind) in cases {
        let message = err.to_string();
        let dto = InstallUpdateErrorResponse::from(err);
        assert_eq!(dto.kind, kind);
        assert_eq!(dto.message, message);
    }
}

#[test]
fn tp_f4_signature_errors_map_to_install_failed_others_to_download_failed() {
    use tauri_plugin_updater::Error as E;
    assert!(matches!(
        map_download_error(E::SignatureUtf8("bad".into())),
        InstallUpdateError::InstallFailed(_)
    ));
    assert!(matches!(
        map_download_error(E::Network("down".into())),
        InstallUpdateError::DownloadFailed(_)
    ));
    assert!(matches!(
        map_download_error(E::Io(std::io::Error::other("disk"))),
        InstallUpdateError::DownloadFailed(_)
    ));
}

// ===== spec.md#tp-concurrency (I-U10) =====

#[test]
fn tp_g1_install_phase_transitions() {
    use InstallPhase::*;
    assert_eq!(Idle.begin(), Some(Installing));
    assert_eq!(Installing.begin(), None);
    assert_eq!(Installed.begin(), None);
    assert_eq!(Installing.finish(true), Installed);
    assert_eq!(Installing.finish(false), Idle);
    assert_eq!(Idle.finish(true), Idle);
    assert_eq!(Installed.finish(false), Installed);
}

#[test]
fn tp_g2_install_while_installing_is_rejected_without_touching_updater() {
    let state = Arc::new(InMemoryInstallState::new());
    // 別 UI からの install が実行中
    assert!(state.try_begin());

    let (result, fetches, installs) =
        run_with(&state, "0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::Ok);

    assert_eq!(result, Err(InstallUpdateError::InstallInProgress));
    assert_eq!((fetches, installs), (0, 0));
    // 実行中の install の状態は変えない
    assert_eq!(state.phase(), InstallPhase::Installing);
}

#[test]
fn tp_g3_success_keeps_installed_until_restart_and_rejects_reinstall() {
    let state = Arc::new(InMemoryInstallState::new());

    let (first, _, _) = run_with(&state, "0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::Ok);
    assert_eq!(first, Ok(v("0.2.5")));
    assert_eq!(state.phase(), InstallPhase::Installed);

    let (second, fetches, installs) =
        run_with(&state, "0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::Ok);
    assert_eq!(second, Err(InstallUpdateError::InstallInProgress));
    assert_eq!((fetches, installs), (0, 0));
}

#[test]
fn tp_g4_failure_releases_the_slot_for_retry() {
    let state = Arc::new(InMemoryInstallState::new());
    for (fetch, outcome) in [
        (
            Err(InstallUpdateError::CheckFailed("net".into())),
            InstallOutcome::Ok,
        ),
        (Ok("0.2.4"), InstallOutcome::Ok),
        (Ok("0.2.5"), InstallOutcome::DownloadFails),
        (Ok("0.2.5"), InstallOutcome::InstallFails),
    ] {
        let (result, _, _) = run_with(&state, "0.2.4", APPIMAGE, fetch, outcome);
        assert!(result.is_err());
        assert_eq!(state.phase(), InstallPhase::Idle);
    }

    let (retry, _, installs) = run_with(&state, "0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::Ok);
    assert_eq!(retry, Ok(v("0.2.5")));
    assert_eq!(installs, 1);
}

#[test]
fn tp_g5_unbundled_does_not_take_the_slot() {
    let state = Arc::new(InMemoryInstallState::new());

    let (result, _, _) = run_with(
        &state,
        "0.2.4",
        InstallationKind::Unbundled,
        Ok("0.2.5"),
        InstallOutcome::Ok,
    );

    assert_eq!(result, Err(InstallUpdateError::UnsupportedInstallation));
    assert_eq!(state.phase(), InstallPhase::Idle);
}

/// download 中に止まる installer (thread 間で使うため Send な実装)。
struct BlockingInstaller {
    started: mpsc::Sender<()>,
    release: Arc<std::sync::Mutex<mpsc::Receiver<()>>>,
}

struct BlockingPending {
    release: Arc<std::sync::Mutex<mpsc::Receiver<()>>>,
}

impl PendingUpdate for BlockingPending {
    fn version_string(&self) -> &str {
        "0.2.5"
    }
    fn download_and_install(self: Box<Self>) -> Result<(), InstallUpdateError> {
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .expect("test should release the blocked install");
        Ok(())
    }
}

impl UpdateInstaller for BlockingInstaller {
    fn fetch_pending(&self) -> Result<Box<dyn PendingUpdate>, InstallUpdateError> {
        self.started.send(()).unwrap();
        Ok(Box::new(BlockingPending {
            release: self.release.clone(),
        }))
    }
}

#[test]
fn tp_g6_concurrent_requests_from_two_threads_run_only_one_install() {
    let state = Arc::new(InMemoryInstallState::new());
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let installer = BlockingInstaller {
        started: started_tx,
        release: Arc::new(std::sync::Mutex::new(release_rx)),
    };

    // 1 本目 (例: toast) は download 中に止まる
    let first_state = state.clone();
    let first = thread::spawn(move || {
        InstallUpdateUseCase::new(installer, first_state).execute(cmd("0.2.4", APPIMAGE))
    });
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first install should start");

    // 2 本目 (例: 設定モーダル) は拒否され、updater に触れない
    let (second, fetches, installs) =
        run_with(&state, "0.2.4", APPIMAGE, Ok("0.2.5"), InstallOutcome::Ok);
    assert_eq!(second, Err(InstallUpdateError::InstallInProgress));
    assert_eq!((fetches, installs), (0, 0));

    release_tx.send(()).unwrap();
    assert_eq!(first.join().unwrap(), Ok(v("0.2.5")));
    assert_eq!(state.phase(), InstallPhase::Installed);
}
