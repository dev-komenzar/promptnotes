//! Tests for slice `change-sort-order`.
//!
//! Spec: `.ori/slices/change-sort-order/spec.md#test-perspectives`.
//!
//! 設計メモ:
//! - 本 slice は **NoteFeed と Settings を同時に touch する唯一の slice** (Customer-Supplier の逆流)
//! - PersistError は `shared::types::PersistError` を利用 (ori-hpo.8 / C-CSO6)
//! - TP-CS1: UseCase の type 構造で「NoteFeed + SettingsRepository 両方を扱う」事を type-level に固定

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;

use crate::note_feed::shared::types::NoteFeed;
use crate::user_preferences::shared::ports::{EventBus, SettingsRepository};
use crate::user_preferences::shared::types::{
    PersistError, Settings, SettingsEvent, SortDirection, SortField, SortOrder, StorageDir, Theme,
};

use super::application::ChangeSortOrderUseCase;
use super::domain::{ChangeSortOrderCommand, ChangeSortOrderError};

// ===== test doubles =====

struct FakeRepo {
    current: RefCell<Settings>,
    saved: RefCell<Vec<Settings>>,
    save_fail: RefCell<Option<io::ErrorKind>>,
}

impl FakeRepo {
    fn new(initial: Settings) -> Self {
        Self {
            current: RefCell::new(initial),
            saved: RefCell::new(Vec::new()),
            save_fail: RefCell::new(None),
        }
    }
    fn fail_save(&self, kind: io::ErrorKind) {
        *self.save_fail.borrow_mut() = Some(kind);
    }
    fn save_count(&self) -> usize {
        self.saved.borrow().len()
    }
    fn last_saved(&self) -> Option<Settings> {
        self.saved.borrow().last().cloned()
    }
    fn current(&self) -> Settings {
        self.current.borrow().clone()
    }
}

impl SettingsRepository for FakeRepo {
    fn load(&self) -> Settings {
        self.current.borrow().clone()
    }
    fn save(&self, settings: &Settings) -> io::Result<()> {
        if let Some(kind) = self.save_fail.borrow_mut().take() {
            return Err(io::Error::new(kind, "fake repo save failure"));
        }
        self.saved.borrow_mut().push(settings.clone());
        *self.current.borrow_mut() = settings.clone();
        Ok(())
    }
}

struct RcRepo(Rc<FakeRepo>);
impl SettingsRepository for RcRepo {
    fn load(&self) -> Settings {
        self.0.load()
    }
    fn save(&self, s: &Settings) -> io::Result<()> {
        self.0.save(s)
    }
}

struct FakeBus {
    published: RefCell<Vec<SettingsEvent>>,
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
    fn events(&self) -> Vec<SettingsEvent> {
        self.published.borrow().clone()
    }
}

impl EventBus for FakeBus {
    fn publish(&self, e: SettingsEvent) {
        self.published.borrow_mut().push(e);
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: SettingsEvent) {
        self.0.publish(e)
    }
}

fn config_path() -> PathBuf {
    PathBuf::from("/tmp/promptnotes-test-config/settings.json")
}

fn initial_settings_with(sort: SortOrder) -> Settings {
    Settings::new(
        StorageDir::try_from(PathBuf::from("/some/abs")).unwrap(),
        Theme::System,
        sort,
    )
}

type Rig = (
    ChangeSortOrderUseCase<RcRepo, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
);

fn rig_with_current_sort(current_sort: SortOrder) -> Rig {
    let repo = Rc::new(FakeRepo::new(initial_settings_with(current_sort)));
    let bus = Rc::new(FakeBus::new());
    let uc = ChangeSortOrderUseCase::new(RcRepo(repo.clone()), RcBus(bus.clone()), config_path());
    (uc, repo, bus)
}

fn order(field: SortField, direction: SortDirection) -> SortOrder {
    SortOrder::new(field, direction)
}

// ===== TP-H*: happy path =====

/// spec.md#tp-happy TP-H1 — sort 更新が NoteFeed に反映
#[test]
fn tp_h1_sort_change_is_reflected_in_returned_feed() {
    let current = order(SortField::CreatedAt, SortDirection::Desc);
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, _r, _b) = rig_with_current_sort(current);

    let feed = NoteFeed::empty(); // sort = default (CreatedAt, Desc) — but repo current は same
    let result = uc
        .execute(feed, ChangeSortOrderCommand { new_sort })
        .expect("happy path should succeed");

    assert_eq!(result.sort(), new_sort, "TP-H1: NoteFeed.sort updated");
}

/// spec.md#tp-happy TP-H2 — save 1 回 + 保存内容 一致
#[test]
fn tp_h2_save_called_once_with_new_sort() {
    let current = order(SortField::CreatedAt, SortDirection::Desc);
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, _b) = rig_with_current_sort(current);

    let _ = uc
        .execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort })
        .unwrap();

    assert_eq!(repo.save_count(), 1, "TP-H2: save called once");
    let saved = repo.last_saved().unwrap();
    assert_eq!(
        saved.sort_preference(),
        new_sort,
        "TP-H2: saved sort matches"
    );
}

/// spec.md#tp-happy TP-H3 — SortPreferenceChanged 1 件 publish
#[test]
fn tp_h3_event_published_exactly_once() {
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, _r, bus) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));

    let _ = uc
        .execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort })
        .unwrap();

    assert_eq!(bus.count(), 1, "TP-H3: exactly one event");
    match bus.events().into_iter().next().unwrap() {
        SettingsEvent::SortPreferenceChanged { new_sort: ns } => {
            assert_eq!(ns, new_sort, "TP-H3: event payload");
        }
        other => panic!("TP-H3: expected SortPreferenceChanged, got {:?}", other),
    }
}

/// spec.md#tp-happy TP-H4 — filter は touch されない (C-CSO5 直交性)
#[test]
fn tp_h4_filter_is_preserved() {
    use crate::note_capture::shared::types::Tag;
    use crate::note_feed::shared::types::{DateRangeFilter, FeedFilter};

    let (uc, _r, _b) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));

    let filter = FeedFilter::initial()
        .with_date_range(DateRangeFilter::Last7Days)
        .with_tag(Some(Tag::new("coding").unwrap()));
    let feed = NoteFeed::empty().with_filter(filter.clone());

    let result = uc
        .execute(
            feed,
            ChangeSortOrderCommand {
                new_sort: order(SortField::UpdatedAt, SortDirection::Asc),
            },
        )
        .unwrap();

    assert_eq!(result.filter(), &filter, "TP-H4: filter unchanged");
}

// ===== TP-N*: no-op =====

/// spec.md#tp-noop TP-N1 — 同値入力 → no-op (C-CSO1)
#[test]
fn tp_n1_same_sort_is_noop() {
    let same = order(SortField::CreatedAt, SortDirection::Desc);
    let (uc, repo, bus) = rig_with_current_sort(same);

    let _ = uc
        .execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort: same })
        .unwrap();

    assert_eq!(repo.save_count(), 0, "TP-N1: no save on same value");
    assert_eq!(bus.count(), 0, "TP-N1: no event on same value");
}

/// spec.md#tp-noop TP-N2 — 戻り値 NoteFeed が入力と同値
#[test]
fn tp_n2_noop_returns_input_feed_equivalent() {
    let same = order(SortField::CreatedAt, SortDirection::Desc);
    let (uc, _r, _b) = rig_with_current_sort(same);

    let input = NoteFeed::empty();
    let result = uc
        .execute(input.clone(), ChangeSortOrderCommand { new_sort: same })
        .unwrap();

    assert_eq!(result, input, "TP-N2: feed unchanged on no-op");
}

// ===== TP-E*: PersistError =====

/// spec.md#tp-persist-error TP-E1 — save 失敗 → Err(ChangeSortOrderError::PersistError)
#[test]
fn tp_e1_save_failure_returns_persist_error() {
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, _b) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));
    repo.fail_save(io::ErrorKind::PermissionDenied);

    let result = uc.execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort });

    assert!(
        matches!(result, Err(PersistError { .. })),
        "TP-E1: persist failure surfaces as PersistError"
    );
}

/// spec.md#tp-persist-error TP-E2 — persist 失敗時 event 0 件 (C-CSO3)
#[test]
fn tp_e2_persist_failure_publishes_no_event() {
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, bus) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));
    repo.fail_save(io::ErrorKind::Other);

    let _ = uc.execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort });

    assert_eq!(bus.count(), 0, "TP-E2: no event on PersistError");
}

/// spec.md#tp-persist-error TP-E3 — persist 失敗時、mock の current() は元の値
#[test]
fn tp_e3_persist_failure_leaves_repo_state_unchanged() {
    let original = order(SortField::CreatedAt, SortDirection::Desc);
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, _b) = rig_with_current_sort(original);
    repo.fail_save(io::ErrorKind::Other);

    let _ = uc.execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort });

    assert_eq!(
        repo.current().sort_preference(),
        original,
        "TP-E3: repo current unchanged on save failure"
    );
}

/// spec.md#tp-persist-error TP-E4 — error 型が shared PersistError と同一 (C-CSO6 type-level)
///
/// ori-hpo.8: `ChangeSortOrderError` は `shared::types::PersistError` の alias。
/// `UpdateSettingsError::PersistError(PersistError)` と同じ inner 型を参照する。
#[test]
fn tp_e4_error_type_is_shared_persist_error() {
    // type-level: ChangeSortOrderError と PersistError が同一型なら、
    // 関数ポインタとして互換 cast できる。shared 層の型を reuse できているかを
    // compile-time に固定。
    let _: fn(PersistError) -> ChangeSortOrderError = |e| e;
    let _: fn(ChangeSortOrderError) -> PersistError = |e| e;
}

// ===== TP-A*: atomic transaction =====

/// spec.md#tp-atomic TP-A1 — result.sort == saved sort_preference (C-CSO2)
#[test]
fn tp_a1_result_sort_equals_saved_sort_preference() {
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, _b) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));

    let result = uc
        .execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort })
        .unwrap();

    let saved = repo.last_saved().unwrap();
    assert_eq!(
        result.sort(),
        saved.sort_preference(),
        "TP-A1: NoteFeed.sort == Settings.sort_preference (atomic)"
    );
}

/// spec.md#tp-atomic TP-A2 — event payload new_sort == saved sort_preference
#[test]
fn tp_a2_event_payload_matches_saved_value() {
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, bus) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));

    let _ = uc
        .execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort })
        .unwrap();

    let saved = repo.last_saved().unwrap();
    match bus.events().into_iter().next().unwrap() {
        SettingsEvent::SortPreferenceChanged { new_sort: ns } => {
            assert_eq!(ns, saved.sort_preference());
        }
        other => panic!("TP-A2: unexpected event {:?}", other),
    }
}

// ===== TP-CS*: Customer-Supplier 逆流 =====

/// spec.md#tp-cs-reverse TP-CS1 — UseCase が NoteFeed + SettingsRepository 両方を扱う (type-level)
///
/// execute シグネチャに NoteFeed と Command を取り、SettingsRepository / EventBus を保持する構造を
/// 関数ポインタ束縛で固定する。
#[test]
fn tp_cs1_execute_takes_note_feed_and_command_returning_result() {
    type ExecuteFn = fn(
        &ChangeSortOrderUseCase<RcRepo, RcBus>,
        NoteFeed,
        ChangeSortOrderCommand,
    ) -> Result<NoteFeed, ChangeSortOrderError>;
    let _: ExecuteFn = ChangeSortOrderUseCase::execute;
}

/// spec.md#tp-cs-reverse TP-CS2 — NoteFeed と Settings に 1:1 で適用
#[test]
fn tp_cs2_sort_change_applies_to_both_aggregates_one_to_one() {
    let new_sort = order(SortField::UpdatedAt, SortDirection::Asc);
    let (uc, repo, _b) = rig_with_current_sort(order(SortField::CreatedAt, SortDirection::Desc));

    let result = uc
        .execute(NoteFeed::empty(), ChangeSortOrderCommand { new_sort })
        .unwrap();

    // 1: NoteFeed 側
    assert_eq!(result.sort(), new_sort);
    // 1: Settings 側
    assert_eq!(repo.current().sort_preference(), new_sort);
    // 1:1 — どちらも new_sort
}
