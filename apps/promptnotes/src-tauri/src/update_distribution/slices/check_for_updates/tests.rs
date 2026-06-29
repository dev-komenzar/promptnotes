//! Tests for slice `check-for-updates`.
//!
//! Spec: `.ori/slices/check-for-updates/spec.md#test-perspectives`.
//!
//! 設計メモ:
//! - `UpdaterPort` を inject。production impl (`TauriUpdaterPort`) は ori-6l4 release infra 完了後に別 issue で追加。
//! - GREEN 化は phase 4 (impl-green) の責務。RED 段階では未実装 module の path 解決失敗を期待する。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use proptest::prelude::*;

use crate::update_distribution::shared::ports::{EventBus, RawRelease, UpdaterPort};
use crate::update_distribution::shared::types::{
    NewVersionDetected, UpdateChannel, UpdateError, Version,
};

use super::application::CheckForUpdatesUseCase;
use super::domain::CheckForUpdatesCommand;

// ===== test doubles =====

/// `UpdaterPort` の test double。`with_response` で挙動 inject、呼出回数を観測。
struct FakeUpdater {
    response: RefCell<Option<Result<RawRelease, UpdateError>>>,
    call_count: Cell<usize>,
}

impl FakeUpdater {
    fn with_response(result: Result<RawRelease, UpdateError>) -> Self {
        Self {
            response: RefCell::new(Some(result)),
            call_count: Cell::new(0),
        }
    }

    fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl UpdaterPort for FakeUpdater {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError> {
        self.call_count.set(self.call_count.get() + 1);
        self.response
            .borrow_mut()
            .take()
            .expect("FakeUpdater response was already consumed (test should set per call)")
    }
}

struct RcUpdater(Rc<FakeUpdater>);
impl UpdaterPort for RcUpdater {
    fn fetch_latest_release(&self) -> Result<RawRelease, UpdateError> {
        self.0.fetch_latest_release()
    }
}

/// `EventBus` の test double。publish 履歴を保持。
struct FakeBus {
    published: RefCell<Vec<NewVersionDetected>>,
}

impl FakeBus {
    fn new() -> Self {
        Self {
            published: RefCell::new(Vec::new()),
        }
    }

    fn count(&self) -> usize {
        self.published.borrow().len()
    }

    fn events(&self) -> Vec<NewVersionDetected> {
        self.published.borrow().clone()
    }
}

impl EventBus for FakeBus {
    fn publish(&self, event: NewVersionDetected) {
        self.published.borrow_mut().push(event);
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: NewVersionDetected) {
        self.0.publish(e)
    }
}

fn current_version() -> Version {
    Version::from_str("0.3.1").expect("0.3.1 is valid semver")
}

fn raw(version: &str, url: &str, notes: &str) -> RawRelease {
    RawRelease {
        version_string: version.to_string(),
        url: url.to_string(),
        notes: notes.to_string(),
    }
}

type Rig = (
    CheckForUpdatesUseCase<RcUpdater, RcBus>,
    Rc<FakeUpdater>,
    Rc<FakeBus>,
);

fn rig(updater_response: Result<RawRelease, UpdateError>) -> Rig {
    let updater = Rc::new(FakeUpdater::with_response(updater_response));
    let bus = Rc::new(FakeBus::new());
    let uc = CheckForUpdatesUseCase::new(RcUpdater(updater.clone()), RcBus(bus.clone()));
    (uc, updater, bus)
}

// ===== TP-N*: happy path — 新版検出 =====

/// spec.md#tp-new-version TP-N1 — current 0.3.1 + latest 0.4.0 → latest_release = Some(0.4.0)
#[test]
fn tp_n1_new_version_returns_some_release() {
    let (uc, _u, _b) = rig(Ok(raw(
        "0.4.0",
        "https://github.com/x/y/releases/tag/v0.4.0",
        "release notes",
    )));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    let release = channel
        .latest_release()
        .expect("TP-N1: latest_release must be Some");
    assert_eq!(
        release.version(),
        &Version::from_str("0.4.0").unwrap(),
        "TP-N1: release.version == 0.4.0"
    );
}

/// spec.md#tp-new-version TP-N2 — NewVersionDetected 1 件 publish
#[test]
fn tp_n2_new_version_publishes_exactly_one_event() {
    let (uc, _u, bus) = rig(Ok(raw(
        "0.4.0",
        "https://github.com/x/y/releases/tag/v0.4.0",
        "",
    )));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(bus.count(), 1, "TP-N2: exactly one NewVersionDetected");
}

/// spec.md#tp-new-version TP-N3 — event payload 一致
#[test]
fn tp_n3_event_payload_matches_release() {
    let (uc, _u, bus) = rig(Ok(raw(
        "0.4.0",
        "https://github.com/x/y/releases/tag/v0.4.0",
        "Release notes here",
    )));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    let event = bus.events().into_iter().next().unwrap();
    assert_eq!(event.current_version, current_version());
    assert_eq!(event.latest_version, Version::from_str("0.4.0").unwrap());
    assert_eq!(
        event.release_url,
        "https://github.com/x/y/releases/tag/v0.4.0"
    );
    assert_eq!(event.release_notes, "Release notes here");
}

// ===== TP-U*: UpToDate (同一版) =====

/// spec.md#tp-no-event TP-U1 — 同一版 → latest_release = None
#[test]
fn tp_u1_up_to_date_returns_none() {
    let (uc, _u, _b) = rig(Ok(raw("0.3.1", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert!(
        channel.latest_release().is_none(),
        "TP-U1: up-to-date must normalize to None (I-U2)"
    );
}

/// spec.md#tp-no-event TP-U2 — 同一版で event 0 件
#[test]
fn tp_u2_up_to_date_publishes_no_event() {
    let (uc, _u, bus) = rig(Ok(raw("0.3.1", "https://example.com", "")));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(bus.count(), 0, "TP-U2: no event on up-to-date (I-U2)");
}

// ===== TP-O*: OlderVersion (古い版) =====

/// spec.md#tp-no-event TP-O1 — 古い版 → latest_release = None
#[test]
fn tp_o1_older_version_returns_none() {
    let (uc, _u, _b) = rig(Ok(raw("0.2.0", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert!(
        channel.latest_release().is_none(),
        "TP-O1: older version must normalize to None (I-U2)"
    );
}

/// spec.md#tp-no-event TP-O2 — 古い版で event 0 件
#[test]
fn tp_o2_older_version_publishes_no_event() {
    let (uc, _u, bus) = rig(Ok(raw("0.2.0", "https://example.com", "")));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(bus.count(), 0);
}

// ===== TP-S14-*: silent failure =====

/// spec.md#tp-s14 TP-S14-1 — NetworkError → latest_release = None
#[test]
fn tp_s14_1_network_error_returns_channel_with_none() {
    let (uc, _u, _b) = rig(Err(UpdateError::NetworkError));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert!(
        channel.latest_release().is_none(),
        "TP-S14-1: NetworkError must be silenced to None"
    );
    assert_eq!(
        channel.current_version(),
        &current_version(),
        "TP-S14-1: current_version preserved (I-U1)"
    );
}

/// spec.md#tp-s14 TP-S14-2 — NetworkError で event 0 件
#[test]
fn tp_s14_2_network_error_publishes_no_event() {
    let (uc, _u, bus) = rig(Err(UpdateError::NetworkError));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(bus.count(), 0, "TP-S14-2: no event on network error (S14)");
}

/// spec.md#tp-s14 TP-S14-3 — 戻り型は UpdateChannel (Result でない、type-level)
#[test]
fn tp_s14_3_execute_returns_update_channel_not_result() {
    // 関数ポインタとして束縛できれば戻り型が compile-time に固定される。
    // Result<UpdateChannel, _> では型不一致でこの test が落ちる。
    let _: fn(&CheckForUpdatesUseCase<RcUpdater, RcBus>, CheckForUpdatesCommand) -> UpdateChannel =
        CheckForUpdatesUseCase::execute;
}

/// spec.md#tp-s14 TP-S14-4 — ParseError も silent
#[test]
fn tp_s14_4_parse_error_is_silent() {
    let (uc, _u, bus) = rig(Err(UpdateError::ParseError));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert!(channel.latest_release().is_none());
    assert_eq!(bus.count(), 0);
}

/// spec.md#tp-s14 TP-S14-4b — UpdaterPort が Ok を返したが version_string が parse 不能 → silent
///
/// semver 仕様に合致しない文字列は `Version::from_str` が `ParseError` を返し、
/// `try_execute` の `?` で bubble up、`execute` で握り潰される。
/// (ori-2lm.9 で strict semver 対応に拡張: "0.4.0-rc1" は valid になったため、
///  ここでは真正に不正な文字列を使用する)
#[test]
fn tp_s14_4b_unparseable_version_from_ok_response_is_silent() {
    let (uc, _u, bus) = rig(Ok(raw("not-a-version", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert!(
        channel.latest_release().is_none(),
        "TP-S14-4b: unparseable version_string → None"
    );
    assert_eq!(
        bus.count(),
        0,
        "TP-S14-4b: no event on parse failure via Ok path"
    );
}

/// spec.md#tp-s14 TP-S14-5 — RateLimited も silent
#[test]
fn tp_s14_5_rate_limited_is_silent() {
    let (uc, _u, bus) = rig(Err(UpdateError::RateLimited));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert!(channel.latest_release().is_none());
    assert_eq!(bus.count(), 0);
}

// ===== TP-R*: リトライなし =====

/// spec.md#tp-no-retry TP-R1 — 任意成功 path で UpdaterPort 呼出 1 回
#[test]
fn tp_r1_success_path_calls_updater_exactly_once() {
    let (uc, updater, _b) = rig(Ok(raw("0.4.0", "https://example.com", "")));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(
        updater.call_count(),
        1,
        "TP-R1: UpdaterPort called exactly once"
    );
}

/// spec.md#tp-no-retry TP-R1b — UpToDate / OlderVersion path でも UpdaterPort 呼出は 1 回 (C-CFU4 全 path)
#[test]
fn tp_r1b_non_new_version_paths_also_call_updater_exactly_once() {
    for response in [
        Ok(raw("0.3.1", "https://example.com", "")), // UpToDate
        Ok(raw("0.2.0", "https://example.com", "")), // OlderVersion
    ] {
        let (uc, updater, _b) = rig(response);
        let _ = uc.execute(CheckForUpdatesCommand {
            current_version: current_version(),
        });
        assert_eq!(
            updater.call_count(),
            1,
            "TP-R1b: UpToDate / OlderVersion path も 1 回呼出 (C-CFU4)"
        );
    }
}

/// spec.md#tp-no-retry TP-R2 — NetworkError 後にリトライしない (I-U3)
#[test]
fn tp_r2_failure_path_does_not_retry() {
    let (uc, updater, _b) = rig(Err(UpdateError::NetworkError));

    let _ = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(
        updater.call_count(),
        1,
        "TP-R2: must not retry on failure (I-U3)"
    );
}

// ===== TP-I*: 不変条件 =====

/// spec.md#tp-invariants TP-I1 — current_version 保持 (I-U1 immutable)
#[test]
fn tp_i1_current_version_is_preserved() {
    let (uc, _u, _b) = rig(Ok(raw("0.4.0", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    assert_eq!(channel.current_version(), &current_version());
}

/// spec.md#tp-invariants TP-I2 — latest_release.is_some() → version > current (I-U2 正規化)
#[test]
fn tp_i2_some_release_has_newer_version() {
    let (uc, _u, _b) = rig(Ok(raw("0.4.0", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    if let Some(release) = channel.latest_release() {
        assert!(
            release.version() > channel.current_version(),
            "TP-I2: I-U2 violation"
        );
    } else {
        panic!("TP-I2: expected Some release");
    }
}

/// spec.md#tp-invariants TP-I3 — latest_release.is_none() → event 0 件
#[test]
fn tp_i3_none_release_implies_no_event() {
    for response in [
        Ok(raw("0.3.1", "https://example.com", "")), // UpToDate
        Ok(raw("0.2.0", "https://example.com", "")), // Older
        Err(UpdateError::NetworkError),
        Err(UpdateError::ParseError),
        Err(UpdateError::RateLimited),
    ] {
        let (uc, _u, bus) = rig(response);
        let channel = uc.execute(CheckForUpdatesCommand {
            current_version: current_version(),
        });
        assert!(channel.latest_release().is_none());
        assert_eq!(bus.count(), 0, "TP-I3: None release must imply zero events");
    }
}

// ===== TP-T*: 型レベル =====

/// spec.md#tp-type-level TP-T1 — execute シグネチャに Result を含まない (C-CFU1)
///
/// TP-S14-3 と物理的に同じ確認だが、spec の別 perspective なので個別に書く。
#[test]
fn tp_t1_execute_signature_has_no_result() {
    let _: fn(&CheckForUpdatesUseCase<RcUpdater, RcBus>, CheckForUpdatesCommand) -> UpdateChannel =
        CheckForUpdatesUseCase::execute;
}

// ===== TP-PR*: pre-release / build metadata (ori-2lm.9) =====

/// pre-release 版 (`0.4.0-rc1`) は current (`0.3.1`) より新しければ NewVersion として検出される。
/// ori-2lm.9 で strict semver 対応に拡張したことで "0.4.0-rc1" が parse 可能になった。
#[test]
fn tp_pr1_pre_release_newer_than_current_is_new_version() {
    let (uc, _u, bus) = rig(Ok(raw("0.4.0-rc1", "https://example.com", "rc notes")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    let release = channel
        .latest_release()
        .expect("TP-PR1: 0.4.0-rc1 > 0.3.1 → Some release");
    assert_eq!(
        release.version(),
        &Version::from_str("0.4.0-rc1").unwrap(),
        "TP-PR1: release.version == 0.4.0-rc1"
    );
    assert_eq!(bus.count(), 1, "TP-PR1: pre-release 新版で event 1 件");
}

/// pre-release 版 (`0.4.0-rc1`) は対応する release 版 (`0.4.0`) より小さい (semver 仕様)。
/// current=0.4.0, latest=0.4.0-rc1 → OlderVersion → latest_release = None。
#[test]
fn tp_pr2_pre_release_older_than_release_is_none() {
    let (uc, _u, bus) = rig(Ok(raw("0.4.0-rc1", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: Version::from_str("0.4.0").unwrap(),
    });

    assert!(
        channel.latest_release().is_none(),
        "TP-PR2: 0.4.0-rc1 < 0.4.0 → None (semver pre-release ordering)"
    );
    assert_eq!(bus.count(), 0);
}

/// build metadata (`0.4.0+build123`) は Ord 比較に影響しない (semver 仕様)。
/// current=0.3.1, latest=0.4.0+build123 → NewVersion (0.4.0+build > 0.3.1)。
#[test]
fn tp_pr3_build_metadata_does_not_affect_comparison() {
    let (uc, _u, _b) = rig(Ok(raw("0.4.0+build123", "https://example.com", "")));

    let channel = uc.execute(CheckForUpdatesCommand {
        current_version: current_version(),
    });

    let release = channel
        .latest_release()
        .expect("TP-PR3: 0.4.0+build123 > 0.3.1 → Some release");
    assert_eq!(
        release.version(),
        &Version::from_str("0.4.0+build123").unwrap(),
        "TP-PR3: build metadata を保持したまま parse"
    );
}

// ===== property tests (ori-2lm.9) =====

proptest! {
    /// 任意の (major, minor, patch) に対し Version::from_str が成功する
    #[test]
    fn prop_version_parse_basic(
        major in 0u32..1000,
        minor in 0u32..1000,
        patch in 0u32..1000,
    ) {
        let s = format!("{}.{}.{}", major, minor, patch);
        let v = Version::from_str(&s).expect("basic semver must parse");
        prop_assert_eq!(v, Version::from_str(&s).unwrap());
    }

    /// pre-release 版は対応する release 版より小さい (semver 仕様)
    #[test]
    fn prop_version_prerelease_less_than_release(
        major in 0u32..1000,
        minor in 0u32..1000,
        patch in 0u32..1000,
        pre in "[a-z][a-z0-9]*",
    ) {
        let release = Version::from_str(&format!("{}.{}.{}", major, minor, patch)).unwrap();
        let prerelease =
            Version::from_str(&format!("{}.{}.{}-{}", major, minor, patch, pre)).unwrap();
        prop_assert!(
            prerelease < release,
            "pre-release ({:?}) must be < release ({:?})",
            prerelease,
            release
        );
    }

    /// build metadata の `Ord` 挙作は `semver` crate 1.x 実装に依存する
    /// (2.0 仕様では無視されるべきだが、1.x では比較対象に含まれる)。
    /// 同じ build metadata → 等価。異なる build metadata → 不等価 (1.x 挙動の固定化)。
    #[test]
    fn prop_version_build_metadata_semver1x_behavior(
        major in 0u32..1000,
        minor in 0u32..1000,
        patch in 0u32..1000,
        meta1 in "[a-z][a-z0-9]*",
        meta2 in "[a-z][a-z0-9]*",
    ) {
        let v1 = Version::from_str(&format!("{}.{}.{}+{}", major, minor, patch, meta1)).unwrap();
        let v2 = Version::from_str(&format!("{}.{}.{}+{}", major, minor, patch, meta2)).unwrap();
        if meta1 == meta2 {
            prop_assert_eq!(v1, v2, "same build metadata → equal");
        } else {
            prop_assert_ne!(v1, v2, "semver 1.x: different build metadata → not equal");
        }
    }

    /// Ord の推移性: a > b && b > c => a > c
    #[test]
    fn prop_version_ord_transitivity(
        a in (0u32..100, 0u32..100, 0u32..100),
        b in (0u32..100, 0u32..100, 0u32..100),
        c in (0u32..100, 0u32..100, 0u32..100),
    ) {
        let va = Version::from_str(&format!("{}.{}.{}", a.0, a.1, a.2)).unwrap();
        let vb = Version::from_str(&format!("{}.{}.{}", b.0, b.1, b.2)).unwrap();
        let vc = Version::from_str(&format!("{}.{}.{}", c.0, c.1, c.2)).unwrap();
        if va > vb && vb > vc {
            prop_assert!(va > vc, "transitivity: a > b > c => a > c");
        }
    }
}
