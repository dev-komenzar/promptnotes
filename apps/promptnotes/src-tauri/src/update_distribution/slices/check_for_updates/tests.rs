//! Tests for slice `check-for-updates`.
//!
//! Spec: `.ori/slices/check-for-updates/spec.md#test-points`.
//!
//! 設計メモ:
//! - `UpdaterPort` / `EventBus` は adapter 境界なので test double (`FakeUpdater` / `FakeBus`) を inject。
//! - `NotifiedVersionLedger` は production の `InMemoryUpdateCheckState` を実物で使う
//!   (spec.md#tp-production-fixture)。
//! - 時刻は引数注入 (`SystemTime`) で検証し、実時間で待たない (spec.md#tp-periodic)。

use std::cell::{Cell, RefCell, RefMut};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use proptest::prelude::*;

use crate::update_distribution::shared::ports::{
    EventBus, NotifiedVersionLedger, RawRelease, UpdaterPort,
};
use crate::update_distribution::shared::types::{
    NewVersionDetected, UpdateChannel, UpdateError, Version,
};

use super::application::CheckForUpdatesUseCase;
use super::domain::{
    is_check_due, should_notify, CheckForUpdatesCommand, PERIODIC_CHECK_TICK, UPDATE_CHECK_INTERVAL,
};
use super::infrastructure::InMemoryUpdateCheckState;

// ===== test doubles =====

/// `UpdaterPort` の test double。応答を queue で inject (1 確認 = 1 応答)、呼出回数を観測。
struct FakeUpdater {
    responses: RefCell<VecDeque<Result<RawRelease, UpdateError>>>,
    call_count: Cell<usize>,
}

impl FakeUpdater {
    fn with_responses(results: Vec<Result<RawRelease, UpdateError>>) -> Self {
        Self {
            responses: RefCell::new(results.into()),
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
        let mut queue: RefMut<'_, _> = self.responses.borrow_mut();
        queue
            .pop_front()
            .expect("FakeUpdater response queue exhausted (test should set one per check)")
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

type UseCase = CheckForUpdatesUseCase<Rc<FakeUpdater>, Rc<FakeBus>, Rc<InMemoryUpdateCheckState>>;

type Rig = (UseCase, Rc<FakeUpdater>, Rc<FakeBus>);

/// production fixture: 通知済み state は `InMemoryUpdateCheckState` の実物 (spec.md#tp-production-fixture)。
fn new_state() -> Rc<InMemoryUpdateCheckState> {
    Rc::new(InMemoryUpdateCheckState::new(SystemTime::UNIX_EPOCH))
}

fn rig(updater_response: Result<RawRelease, UpdateError>) -> Rig {
    rig_seq(vec![updater_response], new_state())
}

/// 同じ ledger (state) で複数回確認する rig。応答は確認 1 回につき 1 つ消費する。
fn rig_seq(
    responses: Vec<Result<RawRelease, UpdateError>>,
    state: Rc<InMemoryUpdateCheckState>,
) -> Rig {
    let updater = Rc::new(FakeUpdater::with_responses(responses));
    let bus = Rc::new(FakeBus::new());
    let uc = CheckForUpdatesUseCase::new(updater.clone(), bus.clone(), state);
    (uc, updater, bus)
}

fn cmd(current: &str) -> CheckForUpdatesCommand {
    CheckForUpdatesCommand {
        current_version: v(current),
    }
}

fn v(s: &str) -> Version {
    Version::from_str(s).expect("valid semver in test")
}

fn ok(version: &str) -> Result<RawRelease, UpdateError> {
    Ok(raw(
        version,
        &format!("https://github.com/x/y/releases/tag/v{version}"),
        "",
    ))
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
    #[allow(clippy::type_complexity)] // signature pin: 型を固定すること自体が目的
    let _: fn(&UseCase, CheckForUpdatesCommand) -> UpdateChannel = CheckForUpdatesUseCase::execute;
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
/// oq-version-pre-release の暫定方針「pre-release / build metadata は ParseError で reject」が
/// 発火する path。`parse_release` 内の `Version::from_str("0.4.0-rc1")` が Err を返し、
/// `try_execute` の `?` で bubble up、`execute` で握り潰される。
#[test]
fn tp_s14_4b_unparseable_version_from_ok_response_is_silent() {
    let (uc, _u, bus) = rig(Ok(raw("0.4.0-rc1", "https://example.com", "")));

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

/// spec.md#tp-no-retry TP-R2 — NetworkError 後に同一確認内で再試行しない (I-U3 / C-CFU4)
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
    #[allow(clippy::type_complexity)] // signature pin: 型を固定すること自体が目的
    let _: fn(&UseCase, CheckForUpdatesCommand) -> UpdateChannel = CheckForUpdatesUseCase::execute;
}

// ===== infrastructure: release_url (ori-m2ra.2) =====

/// aggregates.md#update-channel-aggregate — `Release.url` は GitHub Releases page。
/// latest.json の platform アセット URL (api.github.com/.../assets/<id>) を流用しない。
#[test]
fn release_page_url_points_to_github_release_tag_page() {
    use super::infrastructure::release_page_url;

    assert_eq!(
        release_page_url("0.2.3"),
        "https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.3"
    );
    // tag 由来の `v` prefix 付き version でも `vv` にならない
    assert_eq!(
        release_page_url("v0.2.3"),
        "https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.3"
    );
}

// ===== TP-D*: I-U4 / S23 — 通知済みバージョンの再通知抑止 =====

fn published_versions(bus: &FakeBus) -> Vec<Version> {
    bus.events().into_iter().map(|e| e.latest_version).collect()
}

/// spec.md#tp-notified-once TP-D1 — 同一バージョンを 2 回受信 → 2 回目は event 0 件、
/// ただし戻り値は Some(0.2.3) のまま (S23 When 1 / I-U4)
#[test]
fn tp_d1_same_version_is_notified_only_once() {
    let (uc, _u, bus) = rig_seq(vec![ok("0.2.3"), ok("0.2.3")], new_state());

    let first = uc.execute(cmd("0.2.2"));
    assert_eq!(bus.count(), 1, "TP-D1: 1 回目は通知する");
    assert_eq!(
        first.latest_release().map(|r| r.version()),
        Some(&v("0.2.3"))
    );

    let second = uc.execute(cmd("0.2.2"));
    assert_eq!(
        bus.count(),
        1,
        "TP-D1: 通知済みの 0.2.3 は再通知しない (I-U4)"
    );
    assert_eq!(
        second.latest_release().map(|r| r.version()),
        Some(&v("0.2.3")),
        "TP-D1: 抑止しても確認結果は Some(0.2.3) (S23 Then)"
    );
}

/// spec.md#tp-notified-once TP-D2 — 0.2.3 通知済みの後に 0.2.4 → 通知する (S23 When 2)
#[test]
fn tp_d2_newer_version_after_notified_is_notified() {
    let (uc, _u, bus) = rig_seq(vec![ok("0.2.3"), ok("0.2.3"), ok("0.2.4")], new_state());

    for _ in 0..3 {
        let _ = uc.execute(cmd("0.2.2"));
    }

    assert_eq!(
        published_versions(&bus),
        vec![v("0.2.3"), v("0.2.4")],
        "TP-D2: 0.2.3 は 1 回、0.2.4 は未通知なので通知"
    );
}

/// spec.md#tp-notified-once TP-D3 — 0.2.4 通知済みで 0.2.3 (current より新しいが通知済み以下) → 0 件
#[test]
fn tp_d3_version_below_notified_is_suppressed() {
    let (uc, _u, bus) = rig_seq(vec![ok("0.2.4"), ok("0.2.3")], new_state());

    let _ = uc.execute(cmd("0.2.2"));
    let second = uc.execute(cmd("0.2.2"));

    assert_eq!(
        published_versions(&bus),
        vec![v("0.2.4")],
        "TP-D3: 通知済み以下は再通知しない (I-U4「以下」)"
    );
    assert!(
        second.latest_release().is_some(),
        "TP-D3: 0.2.3 > current 0.2.2 なので I-U2 上は Some のまま"
    );
}

/// spec.md#tp-notified-once TP-D4 — 新しい state (= 再起動相当) では再び通知する (I-U4 揮発)
#[test]
fn tp_d4_fresh_state_notifies_again() {
    let (first_uc, _u1, first_bus) = rig_seq(vec![ok("0.2.3")], new_state());
    let _ = first_uc.execute(cmd("0.2.2"));
    assert_eq!(first_bus.count(), 1);

    let (restarted_uc, _u2, restarted_bus) = rig_seq(vec![ok("0.2.3")], new_state());
    let _ = restarted_uc.execute(cmd("0.2.2"));
    assert_eq!(
        restarted_bus.count(),
        1,
        "TP-D4: 再起動後は通知済み記録が消えるので再び通知 (S23 Then)"
    );
}

/// spec.md#tp-notified-once TP-D5 — 通知しなかった確認 (失敗 / UpToDate / Older) は ledger を変えない
#[test]
fn tp_d5_non_notifying_checks_do_not_touch_ledger() {
    let (uc, _u, bus) = rig_seq(
        vec![
            Err(UpdateError::NetworkError),
            ok("0.2.2"), // UpToDate
            ok("0.2.1"), // Older
            Err(UpdateError::ParseError),
            ok("0.2.3"),
        ],
        new_state(),
    );

    for _ in 0..5 {
        let _ = uc.execute(cmd("0.2.2"));
    }

    assert_eq!(
        published_versions(&bus),
        vec![v("0.2.3")],
        "TP-D5: 前段の非通知確認の後でも新版は通知される (C-CFU9)"
    );
}

/// spec.md#tp-notified-once TP-D7 — should_notify 純粋関数
#[test]
fn tp_d7_should_notify_truth_table() {
    assert!(should_notify(&v("0.2.3"), None), "未通知なら通知");
    assert!(
        should_notify(&v("0.2.4"), Some(&v("0.2.3"))),
        "通知済みより新しければ通知"
    );
    assert!(
        !should_notify(&v("0.2.3"), Some(&v("0.2.3"))),
        "同一は通知しない"
    );
    assert!(
        !should_notify(&v("0.2.2"), Some(&v("0.2.3"))),
        "通知済み以下は通知しない"
    );
}

/// spec.md#tp-notified-once — ledger の claim は判定と記録を 1 操作で行う (C-CFU8)
#[test]
fn tp_d8_ledger_claim_records_only_on_success() {
    let state = InMemoryUpdateCheckState::new(SystemTime::UNIX_EPOCH);

    assert!(state.claim(&v("0.2.3")), "初回 claim は true");
    assert!(!state.claim(&v("0.2.3")), "同一 claim は false");
    assert!(
        !state.claim(&v("0.2.2")),
        "以下の claim は false (記録も変えない)"
    );
    assert!(state.claim(&v("0.2.4")), "より新しい claim は true");
    assert!(!state.claim(&v("0.2.3")), "0.2.4 記録後の 0.2.3 は false");
}

fn arb_version() -> impl Strategy<Value = Version> {
    (0u32..4, 0u32..4, 0u32..4).prop_map(|(a, b, c)| Version::new(a, b, c))
}

proptest! {
    /// spec.md#tp-notified-once TP-D6 — 任意のバージョン列を同じ state で確認すると、
    /// publish されたバージョン列は狭義単調増加かつ current より大きい (C-CFU8 / I-U2 / I-U4)
    #[test]
    fn tp_d6_published_versions_strictly_increase(
        current in arb_version(),
        seen in proptest::collection::vec(arb_version(), 0..12),
    ) {
        let responses = seen.iter().map(|ver| ok(&ver.to_string())).collect::<Vec<_>>();
        let (uc, _u, bus) = rig_seq(responses, new_state());
        for _ in 0..seen.len() {
            let _ = uc.execute(CheckForUpdatesCommand { current_version: current.clone() });
        }

        let published = published_versions(&bus);
        for w in published.windows(2) {
            prop_assert!(w[0] < w[1], "published must strictly increase: {:?}", published);
        }
        for p in &published {
            prop_assert!(p > &current, "published {p:?} must be > current {current:?}");
        }
        // 見えた中で current より大きい最大値は必ず通知されている (取りこぼしなし)
        if let Some(max_newer) = seen.iter().filter(|s| *s > &current).max() {
            prop_assert_eq!(published.last(), Some(max_newer));
        } else {
            prop_assert!(published.is_empty());
        }
    }
}

// ===== TP-P*: I-U3 — 24 時間周期の判定 =====

const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const SEC: Duration = Duration::from_secs(1);

fn t0() -> SystemTime {
    // 任意の基準時刻 (2026-10-25 付近)。UNIX_EPOCH 近傍の underflow を避ける。
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000)
}

/// spec.md#tp-periodic TP-P1 — 周期 24h / tick 1h の定数
#[test]
fn tp_p1_interval_and_tick_constants() {
    assert_eq!(UPDATE_CHECK_INTERVAL, DAY, "TP-P1: I-U3 の周期は 24 時間");
    assert_eq!(
        PERIODIC_CHECK_TICK, HOUR,
        "TP-P1: due 判定 tick は 1 時間 (C-CFU11)"
    );
}

/// spec.md#tp-periodic TP-P2 — 境界: 24h-1s は not due、24h / 24h+1s は due
#[test]
fn tp_p2_due_boundary() {
    assert!(!is_check_due(t0(), t0() + DAY - SEC));
    assert!(is_check_due(t0(), t0() + DAY));
    assert!(is_check_due(t0(), t0() + DAY + SEC));
}

/// spec.md#tp-periodic TP-P3 — wall clock 巻き戻り (now < last) は due
#[test]
fn tp_p3_clock_going_backwards_is_due() {
    assert!(
        is_check_due(t0(), t0() - SEC),
        "TP-P3: 巻き戻りは due (C-CFU11)"
    );
}

/// spec.md#tp-periodic TP-P4 — 未確認の state はプロセス起動時刻を基準にする
#[test]
fn tp_p4_unchecked_state_uses_process_start() {
    let state = InMemoryUpdateCheckState::new(t0());
    assert!(!state.is_due(t0()), "起動直後は not due");
    assert!(!state.is_due(t0() + DAY - SEC));
    assert!(state.is_due(t0() + DAY), "起動から 24h 後は due");
}

/// spec.md#tp-periodic TP-P5 — begin_check で前回確認時刻が更新される (起動時 invoke / 定期共通)
#[test]
fn tp_p5_begin_check_resets_baseline() {
    let state = InMemoryUpdateCheckState::new(t0());
    let checked_at = t0() + 3 * HOUR;
    state.begin_check(checked_at);

    assert!(
        !state.is_due(checked_at + DAY - SEC),
        "前回確認から 24h 未満は not due"
    );
    assert!(state.is_due(checked_at + DAY), "前回確認から 24h で due");
}

/// spec.md#tp-periodic TP-P6 — begin_due_check は due のときだけ true + 更新、not due は false + 不変
#[test]
fn tp_p6_begin_due_check_only_when_due() {
    let state = InMemoryUpdateCheckState::new(t0());

    assert!(!state.begin_due_check(t0() + HOUR), "not due → false");
    assert!(
        state.is_due(t0() + DAY),
        "not due の begin_due_check は基準を変えない"
    );

    let due_at = t0() + DAY + HOUR;
    assert!(state.begin_due_check(due_at), "due → true");
    assert!(
        !state.is_due(due_at + HOUR),
        "true を返したら基準が due_at に更新される"
    );
    assert!(state.is_due(due_at + DAY));
}

proptest! {
    /// spec.md#tp-periodic TP-P7 — is_check_due(last, last + elapsed) == (elapsed >= 24h)
    #[test]
    fn tp_p7_due_iff_elapsed_at_least_interval(elapsed_secs in 0u64..(3 * 24 * 60 * 60)) {
        let elapsed = Duration::from_secs(elapsed_secs);
        prop_assert_eq!(is_check_due(t0(), t0() + elapsed), elapsed >= DAY);
    }
}

// ===== TP-S14-6: 失敗時はログ出力のみ (S14 Then「UI 通知なし、ログ出力のみ」/ C-CFU3) =====

mod log_capture {
    //! `log` crate の出力を test thread ごとに捕捉する logger (adapter 境界の test double)。
    use std::cell::RefCell;
    use std::sync::Once;

    thread_local! {
        static CAPTURED: RefCell<Vec<(log::Level, String)>> = const { RefCell::new(Vec::new()) };
    }

    struct CaptureLogger;

    impl log::Log for CaptureLogger {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            CAPTURED.with(|c| {
                c.borrow_mut()
                    .push((record.level(), record.args().to_string()))
            });
        }

        fn flush(&self) {}
    }

    static LOGGER: CaptureLogger = CaptureLogger;
    static INIT: Once = Once::new();

    /// logger を (プロセスで 1 回) 登録し、この thread の捕捉バッファを空にする。
    pub fn start() {
        INIT.call_once(|| {
            log::set_logger(&LOGGER).expect("no other logger is installed in unit tests");
            log::set_max_level(log::LevelFilter::Trace);
        });
        CAPTURED.with(|c| c.borrow_mut().clear());
    }

    pub fn take() -> Vec<(log::Level, String)> {
        CAPTURED.with(|c| std::mem::take(&mut *c.borrow_mut()))
    }
}

/// spec.md#tp-s14 TP-S14-6 — 失敗 (Network / Parse / RateLimited / parse 不能) は warn ログを 1 件出す
#[test]
fn tp_s14_6_failure_is_logged_as_warning() {
    for response in [
        Err(UpdateError::NetworkError),
        Err(UpdateError::ParseError),
        Err(UpdateError::RateLimited),
        Ok(raw("0.4.0-rc1", "https://example.com", "")),
    ] {
        log_capture::start();
        let (uc, _u, bus) = rig(response);

        let channel = uc.execute(cmd("0.3.1"));

        let logs = log_capture::take();
        let warns: Vec<_> = logs
            .iter()
            .filter(|(level, msg)| {
                *level == log::Level::Warn && msg.contains("update check failed")
            })
            .collect();
        assert_eq!(
            warns.len(),
            1,
            "TP-S14-6: 失敗時は warn ログを 1 件 ({logs:?})"
        );
        assert!(
            channel.latest_release().is_none(),
            "TP-S14-6: UI へは伝搬しない"
        );
        assert_eq!(bus.count(), 0, "TP-S14-6: event も発行しない");
    }
}

/// spec.md#tp-s14 TP-S14-7 — 成功 path (NewVersion / UpToDate / 通知済み) では失敗ログを出さない
#[test]
fn tp_s14_7_success_paths_do_not_log_failure() {
    log_capture::start();
    let (uc, _u, _bus) = rig_seq(vec![ok("0.4.0"), ok("0.4.0"), ok("0.3.1")], new_state());

    for _ in 0..3 {
        let _ = uc.execute(cmd("0.3.1"));
    }

    let logs = log_capture::take();
    assert!(
        logs.iter()
            .all(|(_, msg)| !msg.contains("update check failed")),
        "TP-S14-7: 成功 path では失敗ログを出さない ({logs:?})"
    );
}
