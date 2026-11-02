//! Tests for slice `check-for-updates-manually`.
//!
//! Spec: `.ori/slices/check-for-updates-manually/spec.md#test-points`.
//!
//! - `UpdaterPort` は adapter 境界なので test double (`FakeUpdater`) を inject。
//! - 揮発 state 非変更 (TP-N2) は check-for-updates の production state
//!   (`InMemoryUpdateCheckState`) を実物で使って確認する。

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::SystemTime;

use proptest::prelude::*;

use crate::update_distribution::shared::ports::{NotifiedVersionLedger, RawRelease, UpdaterPort};
use crate::update_distribution::shared::types::{UpdateChannel, UpdateError, Version};
use crate::update_distribution::slices::check_for_updates::infrastructure::InMemoryUpdateCheckState;

use super::application::CheckForUpdatesManuallyUseCase;
use super::commands::UpdateCheckErrorResponse;
use super::domain::CheckForUpdatesManuallyCommand;

// ===== test doubles =====

/// `UpdaterPort` の test double。応答を queue で inject (1 確認 = 1 応答)、呼出回数を観測。
struct FakeUpdater {
    responses: RefCell<VecDeque<Result<RawRelease, UpdateError>>>,
    call_count: Cell<usize>,
}

impl FakeUpdater {
    fn with_responses(results: Vec<Result<RawRelease, UpdateError>>) -> Rc<Self> {
        Rc::new(Self {
            responses: RefCell::new(results.into()),
            call_count: Cell::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl UpdaterPort for FakeUpdater {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError> {
        self.call_count.set(self.call_count.get() + 1);
        self.responses
            .borrow_mut()
            .pop_front()
            .expect("FakeUpdater response queue exhausted (test should set one per check)")
    }
}

fn v(s: &str) -> Version {
    Version::from_str(s).expect("valid semver in test")
}

fn ok(version: &str) -> Result<RawRelease, UpdateError> {
    Ok(RawRelease {
        version_string: version.to_string(),
        url: format!("https://github.com/x/y/releases/tag/v{version}"),
        notes: "notes".to_string(),
    })
}

fn cmd(current: &str) -> CheckForUpdatesManuallyCommand {
    CheckForUpdatesManuallyCommand {
        current_version: v(current),
    }
}

fn run(
    current: &str,
    responses: Vec<Result<RawRelease, UpdateError>>,
) -> (Vec<Result<UpdateChannel, UpdateError>>, Rc<FakeUpdater>) {
    let updater = FakeUpdater::with_responses(responses);
    let uc = CheckForUpdatesManuallyUseCase::new(updater.clone());
    let n = updater.responses.borrow().len();
    let results = (0..n).map(|_| uc.execute(cmd(current))).collect();
    (results, updater)
}

fn latest_version(result: &Result<UpdateChannel, UpdateError>) -> Option<String> {
    result
        .as_ref()
        .expect("expected Ok")
        .latest_release()
        .map(|r| r.version().to_string())
}

// ===== 結果の 3 分岐 (spec.md#tp-results) =====

#[test]
fn tp_r1_newer_release_is_returned_as_some() {
    let (results, updater) = run("0.2.4", vec![ok("0.2.5")]);
    assert_eq!(latest_version(&results[0]), Some("0.2.5".to_string()));
    let channel = results[0].as_ref().unwrap();
    assert_eq!(channel.current_version(), &v("0.2.4"));
    let release = channel.latest_release().unwrap();
    assert_eq!(release.url(), "https://github.com/x/y/releases/tag/v0.2.5");
    assert_eq!(release.notes(), "notes");
    assert_eq!(updater.call_count(), 1);
}

#[test]
fn tp_r2_same_or_older_release_is_normalized_to_none() {
    for fetched in ["0.2.5", "0.2.3"] {
        let (results, _) = run("0.2.5", vec![ok(fetched)]);
        assert_eq!(latest_version(&results[0]), None, "fetched {fetched}");
    }
}

#[test]
fn tp_r3_fetch_failure_is_returned_as_err() {
    for err in [UpdateError::NetworkError, UpdateError::RateLimited] {
        let (results, updater) = run("0.2.5", vec![Err(err.clone())]);
        assert_eq!(results[0], Err(err));
        assert_eq!(updater.call_count(), 1, "no retry (C-CFUM1)");
    }
}

#[test]
fn tp_r4_unparseable_version_is_parse_error() {
    let (results, _) = run("0.2.5", vec![ok("0.3.0-rc1")]);
    assert_eq!(results[0], Err(UpdateError::ParseError));
}

proptest! {
    /// TP-R5: `Some` になるのは `fetched > current` のときだけ。fetch は常に 1 回。
    #[test]
    fn tp_r5_some_iff_fetched_is_newer(
        c in (0u64..4, 0u64..4, 0u64..4),
        f in (0u64..4, 0u64..4, 0u64..4),
    ) {
        let current = format!("{}.{}.{}", c.0, c.1, c.2);
        let fetched = format!("{}.{}.{}", f.0, f.1, f.2);
        let (results, updater) = run(&current, vec![ok(&fetched)]);
        let expected = (v(&fetched) > v(&current)).then(|| fetched.clone());
        prop_assert_eq!(latest_version(&results[0]), expected);
        prop_assert_eq!(updater.call_count(), 1);
    }
}

// ===== I-U9: 揮発 state と event (spec.md#tp-no-side-effects) =====

#[test]
fn tp_n1_same_release_is_returned_on_every_check() {
    let (results, updater) = run("0.2.4", vec![ok("0.2.5"), ok("0.2.5")]);
    assert_eq!(latest_version(&results[0]), Some("0.2.5".to_string()));
    assert_eq!(latest_version(&results[1]), Some("0.2.5".to_string()));
    assert_eq!(updater.call_count(), 2);
}

#[test]
fn tp_n2_manual_check_ignores_and_keeps_notified_state() {
    let state = InMemoryUpdateCheckState::new(SystemTime::UNIX_EPOCH);
    // 自動確認で 0.2.5 を通知済みにする
    assert!(state.claim(&v("0.2.5")));

    let (results, _) = run("0.2.4", vec![ok("0.2.5"), ok("0.2.6")]);
    // 通知済みでも結果は返る (I-U9)
    assert_eq!(latest_version(&results[0]), Some("0.2.5".to_string()));
    assert_eq!(latest_version(&results[1]), Some("0.2.6".to_string()));

    // 手動確認は ledger を変えないので、自動確認の判定は手動確認前と同じ:
    // 0.2.5 は通知済みのまま抑止され、手動確認で見た 0.2.6 は未通知のまま通知される
    assert!(!state.claim(&v("0.2.5")));
    assert!(state.claim(&v("0.2.6")));
}

// ===== boundary (spec.md#tp-boundary) =====

#[test]
fn tp_b2_error_dto_kind_matches_variant_name() {
    for (err, kind) in [
        (UpdateError::NetworkError, "NetworkError"),
        (UpdateError::ParseError, "ParseError"),
        (UpdateError::RateLimited, "RateLimited"),
    ] {
        let message = err.to_string();
        let dto = UpdateCheckErrorResponse::from(err);
        assert_eq!(dto.kind, kind);
        assert_eq!(dto.message, message);
    }
}
