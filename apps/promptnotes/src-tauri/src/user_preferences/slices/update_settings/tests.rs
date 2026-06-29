//! Tests for slice `update-settings`.
//!
//! Spec: `.ori/slices/update-settings/spec.md#test-perspectives`.
//!
//! 設計メモ:
//! - `SettingsRepository` / `EventBus` は port として inject。infrastructure は phase 4 で実装。
//! - 本 slice は I-S4 / C-US7 に従い Note の物理移動を行わないため `NoteRepository` を依存に持たない
//!   (依存に持たないこと自体が TP-S11-2 / TP-I3 の保証になる)。
//! - GREEN 化は phase 4 (impl-green) の責務。RED 段階では未実装 module の path 解決失敗を期待する。

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::user_preferences::shared::ports::{EventBus, SettingsRepository};
use crate::user_preferences::shared::types::{
    Settings, SortDirection, SortField, SortOrder, StorageDir, Theme,
};

use super::application::UpdateSettingsUseCase;
use super::domain::{SettingsEvent, UpdateSettingsCommand, UpdateSettingsError};

// ===== test doubles =====

/// 現在の Settings と save 履歴を保持する fake。`save_fail` で 1 回だけ io::Error を仕込める。
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

/// publish 履歴を保持する fake。
struct FakeBus {
    published: RefCell<Vec<SettingsEvent>>,
}

impl FakeBus {
    fn new() -> Self {
        Self {
            published: RefCell::new(Vec::new()),
        }
    }

    fn events(&self) -> Vec<SettingsEvent> {
        self.published.borrow().clone()
    }

    fn count(&self) -> usize {
        self.published.borrow().len()
    }
}

impl EventBus for FakeBus {
    fn publish(&self, event: SettingsEvent) {
        self.published.borrow_mut().push(event);
    }
}

struct RcBus(Rc<FakeBus>);
impl EventBus for RcBus {
    fn publish(&self, e: SettingsEvent) {
        self.0.publish(e)
    }
}

fn config_path() -> PathBuf {
    // settings.json は storage_dir とは独立した OS config dir に置かれる前提 (I-S2)。
    PathBuf::from("/tmp/promptnotes-test-config/settings.json")
}

fn initial_settings() -> Settings {
    Settings::new(
        StorageDir::try_from(PathBuf::from("/old/notes")).expect("/old/notes is absolute"),
        Theme::System,
        SortOrder::new(SortField::CreatedAt, SortDirection::Desc),
    )
}

type Rig = (
    UpdateSettingsUseCase<RcRepo, RcBus>,
    Rc<FakeRepo>,
    Rc<FakeBus>,
);

fn rig() -> Rig {
    let repo = Rc::new(FakeRepo::new(initial_settings()));
    let bus = Rc::new(FakeBus::new());
    let uc = UpdateSettingsUseCase::new(RcRepo(repo.clone()), RcBus(bus.clone()), config_path());
    (uc, repo, bus)
}

// ===== TP-H*: happy path — 単独フィールド更新 =====

/// spec.md#tp-happy TP-H1 — storage_dir のみ更新 → StorageDirChanged 1 件
#[test]
fn tp_h1_storage_dir_only_update_emits_storage_dir_changed() {
    let (uc, repo, bus) = rig();
    let new_dir = PathBuf::from("/new/notes");

    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(new_dir.clone()),
            new_theme: None,
        })
        .expect("storage_dir update should succeed");

    assert_eq!(result.storage_dir().as_path(), new_dir.as_path());
    assert_eq!(repo.save_count(), 1, "TP-H1: save called once");
    assert_eq!(bus.count(), 1, "TP-H1: exactly one event published");
    match bus.events().into_iter().next().unwrap() {
        SettingsEvent::StorageDirChanged {
            old_dir,
            new_dir: nd,
        } => {
            assert_eq!(old_dir, PathBuf::from("/old/notes"));
            assert_eq!(nd, new_dir);
        }
        other => panic!("TP-H1: expected StorageDirChanged, got {:?}", other),
    }
}

/// spec.md#tp-happy TP-H2 — theme のみ更新 → ThemeChanged 1 件
#[test]
fn tp_h2_theme_only_update_emits_theme_changed() {
    let (uc, repo, bus) = rig();

    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: None,
            new_theme: Some(Theme::Dark),
        })
        .expect("theme update should succeed");

    assert!(matches!(result.theme(), Theme::Dark));
    assert_eq!(repo.save_count(), 1);
    assert_eq!(bus.count(), 1);
    match bus.events().into_iter().next().unwrap() {
        SettingsEvent::ThemeChanged { new_theme } => {
            assert!(matches!(new_theme, Theme::Dark));
        }
        other => panic!("TP-H2: expected ThemeChanged, got {:?}", other),
    }
}

// ===== TP-B*: 両フィールド同時更新 =====

/// spec.md#tp-both TP-B1 — 両更新 → 2 件 event、順序 StorageDirChanged → ThemeChanged
#[test]
fn tp_b1_both_fields_update_emits_two_events_in_order() {
    let (uc, repo, bus) = rig();
    let new_dir = PathBuf::from("/new/notes");

    let _ = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(new_dir.clone()),
            new_theme: Some(Theme::Light),
        })
        .expect("both-field update should succeed");

    assert_eq!(
        repo.save_count(),
        1,
        "TP-B1: save called once for combined diff"
    );
    let events = bus.events();
    assert_eq!(events.len(), 2, "TP-B1: two events published");
    assert!(
        matches!(events[0], SettingsEvent::StorageDirChanged { .. }),
        "TP-B1 / C-US5: first event is StorageDirChanged"
    );
    assert!(
        matches!(events[1], SettingsEvent::ThemeChanged { .. }),
        "TP-B1 / C-US5: second event is ThemeChanged"
    );
}

// ===== TP-N*: no-op =====

/// spec.md#tp-noop TP-N1 — 両 None → save 呼ばれない、event 0 (C-US1)
#[test]
fn tp_n1_both_none_is_no_op() {
    let (uc, repo, bus) = rig();
    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: None,
            new_theme: None,
        })
        .expect("no-op should succeed");

    assert_eq!(result, initial_settings());
    assert_eq!(repo.save_count(), 0, "TP-N1: save not called");
    assert_eq!(bus.count(), 0, "TP-N1: no event published");
}

/// spec.md#tp-noop TP-N2 (theme case) — new_theme が現在値と同一 → そのフィールド分の event 0 (C-US2)
#[test]
fn tp_n2_theme_same_value_does_not_emit_event() {
    let (uc, repo, bus) = rig();
    // current.theme == Theme::System、同じ値を指定
    let _ = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: None,
            new_theme: Some(Theme::System),
        })
        .expect("same-value update should succeed");

    assert_eq!(
        repo.save_count(),
        0,
        "TP-N2 (theme): save not called when no diff"
    );
    assert_eq!(
        bus.count(),
        0,
        "TP-N2 (theme): no event when value equals current"
    );
}

/// spec.md#tp-noop TP-N2 (storage_dir case) — new_storage_dir が現在値と同一 → そのフィールド分の event 0 (C-US2)
///
/// `Some(/old/notes)` 単独の同値ケース。TP-N3 (両 field 同値) と対で spec.md#tp-noop を完全カバーする。
#[test]
fn tp_n2_storage_dir_same_value_does_not_emit_event() {
    let (uc, repo, bus) = rig();
    // current.storage_dir == /old/notes、同じ値を指定
    let _ = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(PathBuf::from("/old/notes")),
            new_theme: None,
        })
        .expect("same-value storage_dir update should succeed");

    assert_eq!(
        repo.save_count(),
        0,
        "TP-N2 (storage_dir): save not called when storage_dir equals current"
    );
    assert_eq!(
        bus.count(),
        0,
        "TP-N2 (storage_dir): no event when storage_dir equals current"
    );
}

/// spec.md#tp-noop TP-N3 — 両フィールド同値 → no-op
#[test]
fn tp_n3_both_same_value_is_no_op() {
    let (uc, repo, bus) = rig();
    let _ = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(PathBuf::from("/old/notes")),
            new_theme: Some(Theme::System),
        })
        .expect("both-same update should succeed");

    assert_eq!(repo.save_count(), 0);
    assert_eq!(bus.count(), 0);
}

// ===== TP-S11-*: S11 scenario =====

/// spec.md#tp-s11 TP-S11-1 — Given/When/Then walkthrough
#[test]
fn tp_s11_1_storage_dir_change_persists_and_emits() {
    let (uc, repo, bus) = rig();
    let new_dir = PathBuf::from("/new/path");

    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(new_dir.clone()),
            new_theme: None,
        })
        .expect("S11: storage_dir change succeeds");

    // settings.json への persist
    assert_eq!(repo.save_count(), 1);
    let saved = repo.last_saved().unwrap();
    assert_eq!(saved.storage_dir().as_path(), new_dir.as_path());

    // StorageDirChanged 発行
    let events = bus.events();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SettingsEvent::StorageDirChanged {
            old_dir,
            new_dir: nd,
        } => {
            assert_eq!(old_dir, &PathBuf::from("/old/notes"));
            assert_eq!(nd, &new_dir);
        }
        _ => panic!("S11: expected StorageDirChanged"),
    }

    // result が更新後 Settings
    assert_eq!(result.storage_dir().as_path(), new_dir.as_path());
}

/// spec.md#tp-s11 TP-S11-2 / TP-I3 — use case が NoteRepository に依存しない (I-S4 / C-US7)
///
/// `UpdateSettingsUseCase::new` のシグネチャに NoteRepository / NoteFeed の入る余地がない事を
/// type-level に固定する。
#[test]
fn tp_s11_2_use_case_has_no_note_repository_dependency() {
    let _: fn(RcRepo, RcBus, PathBuf) -> UpdateSettingsUseCase<RcRepo, RcBus> =
        UpdateSettingsUseCase::new;
}

// ===== TP-E*: InvalidPath =====

/// spec.md#tp-invalid-path TP-E1 — 相対パス → InvalidPath、save / event なし (I-S1, C-US3, C-US6)
#[test]
fn tp_e1_relative_storage_dir_returns_invalid_path() {
    let (uc, repo, bus) = rig();

    let result = uc.execute(UpdateSettingsCommand {
        new_storage_dir: Some(PathBuf::from("relative/path")),
        new_theme: None,
    });

    assert!(matches!(
        result,
        Err(UpdateSettingsError::InvalidPath { .. })
    ));
    assert_eq!(repo.save_count(), 0, "TP-E1: no save on InvalidPath");
    assert_eq!(bus.count(), 0, "TP-E1: no event on InvalidPath");
}

/// spec.md#tp-invalid-path TP-E2 — I-S2 違反 (storage_dir が config_path の親) → InvalidPath
///
/// config_path = `/tmp/promptnotes-test-config/settings.json`
/// new_storage_dir = `/tmp/promptnotes-test-config` ← config がこの中にネスト → I-S2 違反
#[test]
fn tp_e2_storage_dir_containing_config_path_is_rejected() {
    let (uc, repo, bus) = rig();

    let result = uc.execute(UpdateSettingsCommand {
        new_storage_dir: Some(PathBuf::from("/tmp/promptnotes-test-config")),
        new_theme: None,
    });

    assert!(
        matches!(result, Err(UpdateSettingsError::InvalidPath { .. })),
        "TP-E2: I-S2 violation must be rejected as InvalidPath"
    );
    assert_eq!(repo.save_count(), 0);
    assert_eq!(bus.count(), 0);
}

/// spec.md#tp-invalid-path TP-E3 — theme も含む場合でも storage_dir 検証が先行して reject (C-US3: partial update なし)
#[test]
fn tp_e3_invalid_storage_dir_blocks_theme_update() {
    let (uc, repo, bus) = rig();

    let result = uc.execute(UpdateSettingsCommand {
        new_storage_dir: Some(PathBuf::from("relative")),
        new_theme: Some(Theme::Dark),
    });

    assert!(matches!(
        result,
        Err(UpdateSettingsError::InvalidPath { .. })
    ));
    assert_eq!(repo.save_count(), 0);
    assert_eq!(bus.count(), 0);
    // current value 不変
    assert!(matches!(repo.load().theme(), Theme::System));
}

// ===== TP-E*: PersistError =====

/// spec.md#tp-persist-error TP-E4 — save 失敗 → PersistError、event 0 件 (C-US4, C-US6)
///
/// `PersistError.path` が use case に渡した `config_path` と一致することを assert して
/// regression に備える (application.rs で `self.config_path.clone()` を埋め込む実装を固定)。
#[test]
fn tp_e4_save_failure_returns_persist_error_with_no_events() {
    let (uc, repo, bus) = rig();
    repo.fail_save(io::ErrorKind::PermissionDenied);

    let result = uc.execute(UpdateSettingsCommand {
        new_storage_dir: Some(PathBuf::from("/new/notes")),
        new_theme: None,
    });

    match result {
        Err(UpdateSettingsError::PersistError(err)) => {
            assert_eq!(
                err.path,
                config_path(),
                "TP-E4: PersistError.path must equal config_path"
            );
            assert_eq!(
                err.cause.kind(),
                io::ErrorKind::PermissionDenied,
                "TP-E4: PersistError.cause preserves original io::ErrorKind"
            );
        }
        other => panic!(
            "TP-E4: expected Err(UpdateSettingsError::PersistError), got {:?}",
            other
        ),
    }
    assert_eq!(bus.count(), 0, "TP-E4: no event on PersistError");
}

/// spec.md#tp-persist-error TP-E5 — persist 失敗時、publish が呼ばれない
#[test]
fn tp_e5_persist_failure_does_not_publish_any_event() {
    let (uc, repo, bus) = rig();
    repo.fail_save(io::ErrorKind::Other);

    let _ = uc.execute(UpdateSettingsCommand {
        new_storage_dir: None,
        new_theme: Some(Theme::Dark),
    });

    assert_eq!(bus.count(), 0);
}

// ===== TP-I*: 不変条件 =====

/// spec.md#tp-invariants TP-I1 — 成功時 result.storage_dir.is_absolute() (I-S1)
#[test]
fn tp_i1_result_storage_dir_is_absolute() {
    let (uc, _repo, _bus) = rig();
    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(PathBuf::from("/another/abs")),
            new_theme: None,
        })
        .expect("update should succeed");
    assert!(result.storage_dir().as_path().is_absolute());
}

/// spec.md#tp-invariants TP-I2 — 成功時 config_path.starts_with(storage_dir) == false (I-S2)
#[test]
fn tp_i2_config_path_not_inside_storage_dir() {
    let (uc, _repo, _bus) = rig();
    let new_dir = PathBuf::from("/new/abs");
    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(new_dir.clone()),
            new_theme: None,
        })
        .expect("update should succeed");
    assert!(
        !config_path().starts_with(result.storage_dir().as_path()),
        "TP-I2: config_path must not be descendant of storage_dir"
    );
    let _ = new_dir;
}

/// spec.md#tp-invariants TP-I2 (boundary) — config_path と同じ親を share する storage_dir は受理される
///
/// TP-E2 は violation 側 (`storage_dir` が `config_path` の親) のみ cover。本 test は
/// `config_path = /tmp/promptnotes-test-config/settings.json` に対して
/// `new_storage_dir = /tmp/promptnotes-test-config/notes` (settings.json と同じ親の子) を
/// 受理する positive 境界を明示化する (I-S2: `config_path.starts_with(storage_dir) == false`)。
#[test]
fn tp_i2_sibling_storage_dir_is_accepted() {
    let (uc, repo, _bus) = rig();
    let new_dir = PathBuf::from("/tmp/promptnotes-test-config/notes");

    let result = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(new_dir.clone()),
            new_theme: None,
        })
        .expect("sibling storage_dir should be accepted (I-S2 boundary)");

    assert_eq!(result.storage_dir().as_path(), new_dir.as_path());
    assert!(
        !config_path().starts_with(result.storage_dir().as_path()),
        "TP-I2 (boundary): config_path must not be descendant of sibling storage_dir"
    );
    assert_eq!(repo.save_count(), 1, "sibling case should persist");
}

// ===== TP-O*: event 順序 =====

/// spec.md#tp-event-order TP-O1 — 両更新時 publish 順 = StorageDirChanged → ThemeChanged (C-US5)
#[test]
fn tp_o1_combined_update_publishes_storage_then_theme() {
    let (uc, _repo, bus) = rig();
    let _ = uc
        .execute(UpdateSettingsCommand {
            new_storage_dir: Some(PathBuf::from("/x/y")),
            new_theme: Some(Theme::Light),
        })
        .expect("update should succeed");

    let events = bus.events();
    assert_eq!(events.len(), 2);
    // 順序が逆ではないこと
    assert!(matches!(events[0], SettingsEvent::StorageDirChanged { .. }));
    assert!(matches!(events[1], SettingsEvent::ThemeChanged { .. }));
}

// helper to make Path unused warning not fire
#[allow(dead_code)]
fn _path_marker(_: &Path) {}
