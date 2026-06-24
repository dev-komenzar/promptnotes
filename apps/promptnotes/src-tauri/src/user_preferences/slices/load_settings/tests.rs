//! Tests for slice `load-settings`.
//!
//! Spec: `.ori/slices/load-settings/spec.md#test-perspectives`.
//!
//! 設計メモ:
//! - 本 slice の戻り値は always `Settings`（Result / Option 露出なし、C-LS1）。
//! - `FileSystem` / `OsDirs` は port として inject。infrastructure は phase 4 で実装。
//! - GREEN 化は phase 4 (impl-green) の責務。RED 段階では未実装 module の path 解決失敗を期待する。

use std::cell::{Cell, RefCell};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use proptest::prelude::*;

use crate::user_preferences::shared::ports::{FileSystem, OsDirs};
use crate::user_preferences::shared::types::{
    Settings, SortDirection, SortField, StorageDir, Theme,
};

use super::application::LoadSettingsUseCase;
use super::domain::LoadSettingsCommand;

// ===== test doubles =====

/// Settings を読む / storage_dir を ensure する fake。
/// `try_read` の返り値は `with_content` で固定する。`ensure_dir` の挙動は
/// `fail_ensure(kind)` で io::Error を 1 回だけ返すように仕込める。
struct FakeFileSystem {
    read_returns: RefCell<Option<String>>,
    ensured_dirs: RefCell<Vec<PathBuf>>,
    ensure_fail: Cell<Option<io::ErrorKind>>,
}

impl FakeFileSystem {
    fn new() -> Self {
        Self {
            read_returns: RefCell::new(None),
            ensured_dirs: RefCell::new(Vec::new()),
            ensure_fail: Cell::new(None),
        }
    }

    fn with_content(content: &str) -> Self {
        let fs = Self::new();
        *fs.read_returns.borrow_mut() = Some(content.into());
        fs
    }

    fn fail_ensure(&self, kind: io::ErrorKind) {
        self.ensure_fail.set(Some(kind));
    }

    fn ensured_paths(&self) -> Vec<PathBuf> {
        self.ensured_dirs.borrow().clone()
    }

    fn ensure_count(&self) -> usize {
        self.ensured_dirs.borrow().len()
    }
}

impl FileSystem for FakeFileSystem {
    fn try_read(&self, _path: &Path) -> Option<String> {
        self.read_returns.borrow().clone()
    }

    fn ensure_dir(&self, path: &Path) -> io::Result<()> {
        if let Some(kind) = self.ensure_fail.take() {
            return Err(io::Error::new(kind, "fake fs failure"));
        }
        self.ensured_dirs.borrow_mut().push(path.to_path_buf());
        Ok(())
    }
}

struct RcFs(Rc<FakeFileSystem>);
impl FileSystem for RcFs {
    fn try_read(&self, p: &Path) -> Option<String> {
        self.0.try_read(p)
    }
    fn ensure_dir(&self, p: &Path) -> io::Result<()> {
        self.0.ensure_dir(p)
    }
}

/// OS 慣習 default を返す fake。呼び出し回数を観測できる。
struct FakeOsDirs {
    default_path: PathBuf,
    call_count: Cell<usize>,
}

impl FakeOsDirs {
    fn new() -> Self {
        Self {
            default_path: PathBuf::from("/tmp/promptnotes-default-storage"),
            call_count: Cell::new(0),
        }
    }

    fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl OsDirs for FakeOsDirs {
    fn default_storage_dir(&self) -> StorageDir {
        self.call_count.set(self.call_count.get() + 1);
        StorageDir::try_from(self.default_path.clone())
            .expect("/tmp path must be absolute on POSIX test runner")
    }
}

struct RcOs(Rc<FakeOsDirs>);
impl OsDirs for RcOs {
    fn default_storage_dir(&self) -> StorageDir {
        self.0.default_storage_dir()
    }
}

fn config_path() -> PathBuf {
    // settings.json は storage_dir と独立した OS config dir に置かれる前提 (I-S2)。
    PathBuf::from("/tmp/promptnotes-test-config/settings.json")
}

type Rig = (
    LoadSettingsUseCase<RcFs, RcOs>,
    Rc<FakeFileSystem>,
    Rc<FakeOsDirs>,
);

fn rig(content: Option<&str>) -> Rig {
    let fs = Rc::new(match content {
        Some(c) => FakeFileSystem::with_content(c),
        None => FakeFileSystem::new(),
    });
    let dirs = Rc::new(FakeOsDirs::new());
    let uc = LoadSettingsUseCase::new(RcFs(fs.clone()), RcOs(dirs.clone()));
    (uc, fs, dirs)
}

// ===== TP-H*: happy path — 完全な settings.json =====

/// spec.md#tp-happy TP-H1 — valid な settings.json (全フィールド) → 内容がそのまま反映
#[test]
fn tp_h1_full_settings_json_is_restored_as_is() {
    let json = r#"{
      "storage_dir": "/abs/notes",
      "theme": "Dark",
      "sort_preference": { "field": "updated_at", "direction": "asc" }
    }"#;
    let (uc, _fs, _dirs) = rig(Some(json));

    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(settings.storage_dir().as_path(), Path::new("/abs/notes"));
    assert!(matches!(settings.theme(), Theme::Dark));
    assert_eq!(settings.sort_preference().field(), SortField::UpdatedAt);
    assert_eq!(settings.sort_preference().direction(), SortDirection::Asc);
}

/// spec.md#tp-happy TP-H2 — happy path で ensure_dir(storage_dir) が 1 回呼ばれる (C-LS5)
#[test]
fn tp_h2_happy_path_calls_ensure_dir_once() {
    let json = r#"{
      "storage_dir": "/abs/notes",
      "theme": "Dark",
      "sort_preference": { "field": "created_at", "direction": "desc" }
    }"#;
    let (uc, fs, _dirs) = rig(Some(json));

    let _ = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        fs.ensure_count(),
        1,
        "TP-H2: ensure_dir called exactly once"
    );
    assert_eq!(
        fs.ensured_paths()[0],
        PathBuf::from("/abs/notes"),
        "TP-H2: ensure_dir is called for storage_dir"
    );
}

/// spec.md#tp-happy TP-H3 — domain event は発行されない (C-LS7)
/// load-settings は EventBus を inject しない設計のため、event 発行経路が型として存在しないことを示す。
/// LoadSettingsUseCase::new は `(FileSystem, OsDirs)` の 2 引数のみで構築できる必要がある。
#[test]
fn tp_h3_use_case_constructor_takes_no_event_bus() {
    let _: fn(RcFs, RcOs) -> LoadSettingsUseCase<RcFs, RcOs> = LoadSettingsUseCase::new;
}

// ===== TP-A*: settings.json 不在 =====

/// spec.md#tp-absent TP-A1 — 不在 → I-S3 全デフォルト
#[test]
fn tp_a1_absent_file_returns_all_defaults() {
    let (uc, _fs, dirs) = rig(None);

    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage"),
        "TP-A1: storage_dir falls back to OsDirs default"
    );
    assert!(matches!(settings.theme(), Theme::System));
    assert_eq!(settings.sort_preference().field(), SortField::CreatedAt);
    assert_eq!(settings.sort_preference().direction(), SortDirection::Desc);
    let _ = dirs;
}

/// spec.md#tp-absent TP-A2 — try_read=None → OsDirs::default_storage_dir が呼ばれる
#[test]
fn tp_a2_absent_file_invokes_os_dirs_default() {
    let (uc, _fs, dirs) = rig(None);

    let _ = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        dirs.call_count(),
        1,
        "TP-A2: OsDirs::default_storage_dir is called once"
    );
}

/// spec.md#tp-absent TP-A3 — 不在時 ensure_dir(default_storage_dir) が呼ばれる (C-LS5)
#[test]
fn tp_a3_absent_file_ensures_default_storage_dir() {
    let (uc, fs, _dirs) = rig(None);

    let _ = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(fs.ensure_count(), 1);
    assert_eq!(
        fs.ensured_paths()[0],
        PathBuf::from("/tmp/promptnotes-default-storage"),
        "TP-A3: ensure_dir runs on the OS default path"
    );
}

// ===== TP-P*: settings.json parse 失敗 =====

/// spec.md#tp-parse-fail TP-P1 — 不正 JSON → I-S3 全デフォルト (C-LS3)
#[test]
fn tp_p1_malformed_json_yields_all_defaults() {
    let (uc, _fs, _dirs) = rig(Some("not a json {"));

    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage")
    );
    assert!(matches!(settings.theme(), Theme::System));
    assert_eq!(settings.sort_preference().field(), SortField::CreatedAt);
}

/// spec.md#tp-parse-fail TP-P2 — top-level array → I-S3 全デフォルト
#[test]
fn tp_p2_array_at_top_level_yields_all_defaults() {
    let (uc, _, _) = rig(Some("[]"));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert!(matches!(settings.theme(), Theme::System));
    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage")
    );
}

/// spec.md#tp-parse-fail TP-P3 — 空文字列 → I-S3 全デフォルト
#[test]
fn tp_p3_empty_string_yields_all_defaults() {
    let (uc, _, _) = rig(Some(""));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert!(matches!(settings.theme(), Theme::System));
}

/// spec.md#tp-parse-fail TP-P4 — `"null"` → I-S3 全デフォルト
#[test]
fn tp_p4_json_null_yields_all_defaults() {
    let (uc, _, _) = rig(Some("null"));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert!(matches!(settings.theme(), Theme::System));
}

// ===== TP-PT*: フィールド単位 fallback (C-LS4, oq-field-level-fallback 暫定採用) =====

/// spec.md#tp-partial TP-PT1 — theme のみ指定 → theme=Dark、他は I-S3 デフォルト
#[test]
fn tp_pt1_theme_only_keeps_theme_and_defaults_rest() {
    let (uc, _, _) = rig(Some(r#"{ "theme": "Dark" }"#));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert!(matches!(settings.theme(), Theme::Dark));
    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage")
    );
    assert_eq!(settings.sort_preference().field(), SortField::CreatedAt);
    assert_eq!(settings.sort_preference().direction(), SortDirection::Desc);
}

/// spec.md#tp-partial TP-PT2 — storage_dir + sort_preference のみ → theme のみ I-S3
#[test]
fn tp_pt2_partial_keeps_provided_and_defaults_missing() {
    let json = r#"{
      "storage_dir": "/abs/x",
      "sort_preference": { "field": "updated_at", "direction": "asc" }
    }"#;
    let (uc, _, _) = rig(Some(json));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert_eq!(settings.storage_dir().as_path(), Path::new("/abs/x"));
    assert_eq!(settings.sort_preference().field(), SortField::UpdatedAt);
    assert_eq!(settings.sort_preference().direction(), SortDirection::Asc);
    assert!(matches!(settings.theme(), Theme::System));
}

/// spec.md#tp-partial TP-PT3 — theme 値不正 → theme のみ I-S3 fallback (oq-field-level-fallback)
#[test]
fn tp_pt3_invalid_theme_value_falls_back_only_theme() {
    let (uc, _, _) = rig(Some(r#"{ "theme": "Invalid", "storage_dir": "/abs/x" }"#));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert!(
        matches!(settings.theme(), Theme::System),
        "TP-PT3: invalid theme falls back to System"
    );
    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/abs/x"),
        "TP-PT3: other fields are preserved"
    );
}

/// spec.md#tp-partial TP-PT4 — nested 欠損 → direction のみ desc 補完
#[test]
fn tp_pt4_nested_missing_field_falls_back() {
    let (uc, _, _) = rig(Some(r#"{ "sort_preference": { "field": "updated_at" } }"#));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert_eq!(settings.sort_preference().field(), SortField::UpdatedAt);
    assert_eq!(
        settings.sort_preference().direction(),
        SortDirection::Desc,
        "TP-PT4: missing direction falls back to I-S3 desc"
    );
}

// ===== TP-M*: storage_dir mkdir =====

/// spec.md#tp-mkdir TP-M1 — 既存ディレクトリでも ensure_dir は呼ばれる (no-op semantics は fs 側)
#[test]
fn tp_m1_existing_dir_still_invokes_ensure_dir() {
    let (uc, fs, _) = rig(None);
    let _ = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert_eq!(fs.ensure_count(), 1);
}

/// spec.md#tp-mkdir TP-M2 — 不在 → ensure_dir が成功記録される (C-LS5)
#[test]
fn tp_m2_missing_dir_records_ensure_success() {
    let (uc, fs, _) = rig(None);
    let _ = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert_eq!(fs.ensured_paths().len(), 1);
}

/// spec.md#tp-mkdir TP-M3 — ensure_dir が io::Error → panic せず Settings を返す (C-LS6)
#[test]
fn tp_m3_ensure_dir_failure_is_silent_and_returns_settings() {
    let (uc, fs, _) = rig(None);
    fs.fail_ensure(io::ErrorKind::PermissionDenied);

    // panic しないこと自体が assertion (catch_unwind は不要)。
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert!(matches!(settings.theme(), Theme::System));
}

/// spec.md#tp-mkdir TP-M4 — TP-M3 でも戻り値 Settings は他 TP と同等の構造
#[test]
fn tp_m4_ensure_dir_failure_does_not_alter_settings_shape() {
    let (uc, fs, _) = rig(None);
    fs.fail_ensure(io::ErrorKind::PermissionDenied);
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage")
    );
    assert_eq!(settings.sort_preference().field(), SortField::CreatedAt);
    assert_eq!(settings.sort_preference().direction(), SortDirection::Desc);
}

// ===== TP-I*: 不変条件 =====

/// spec.md#tp-invariants TP-I2 — 相対パス指定 → storage_dir のみ I-S3 fallback
#[test]
fn tp_i2_relative_storage_dir_falls_back_only_storage_dir() {
    let (uc, _, _) = rig(Some(
        r#"{ "storage_dir": "relative/path", "theme": "Dark" }"#,
    ));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage"),
        "TP-I2: relative path is rejected by I-S1 and replaced with OS default"
    );
    assert!(
        matches!(settings.theme(), Theme::Dark),
        "TP-I2: unrelated fields are preserved"
    );
}

/// spec.md#tp-invariants TP-I3 — config_path は storage_dir 配下にない (I-S2: 循環参照禁止)
///
/// 違反ケース: settings.json が storage_dir の内側にネストする (config_path.starts_with(storage_dir))。
/// このときは impl が I-S3 fallback に降格させる。
#[test]
fn tp_i3_config_path_inside_storage_dir_is_rejected() {
    // config_path = /tmp/promptnotes-test-config/settings.json
    // storage_dir = /tmp/promptnotes-test-config  (config がこの中にある → 違反)
    let json = r#"{ "storage_dir": "/tmp/promptnotes-test-config" }"#;
    let (uc, _, _) = rig(Some(json));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-default-storage"),
        "TP-I3 (I-S2): config_path inside storage_dir must be rejected and fall back to OS default"
    );
}

/// spec.md#tp-invariants TP-I3b — sibling 配置 (config と storage が同じ親) は I-S2 違反ではない
///
/// macOS 慣習: `Application Support/promptnotes/{settings.json, notes/}`。
/// `storage_dir.parent() == config_path.parent()` は許容され、user 指定値がそのまま使われる。
#[test]
fn tp_i3b_sibling_layout_is_allowed() {
    // config_path = /tmp/promptnotes-test-config/settings.json
    // storage_dir = /tmp/promptnotes-test-config/notes  (sibling)
    let json = r#"{ "storage_dir": "/tmp/promptnotes-test-config/notes" }"#;
    let (uc, _, _) = rig(Some(json));
    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        settings.storage_dir().as_path(),
        Path::new("/tmp/promptnotes-test-config/notes"),
        "TP-I3b (I-S2): sibling layout (same parent) must be preserved"
    );
}

/// spec.md#tp-invariants TP-I4 — 冪等性: 同じ入力で 2 回呼出 → 同結果、2 回目は ensure_dir 増えない (C-LS8)
#[test]
fn tp_i4_idempotent_no_double_ensure_dir_on_second_run() {
    let json = r#"{ "theme": "Dark" }"#;
    let (uc, fs, _) = rig(Some(json));

    let first = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });
    let second = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert_eq!(
        first.storage_dir().as_path(),
        second.storage_dir().as_path()
    );
    assert!(matches!(first.theme(), Theme::Dark));
    assert!(matches!(second.theme(), Theme::Dark));
    assert_eq!(
        fs.ensure_count(),
        2,
        "TP-I4: ensure_dir is invoked each call (use case is stateless); idempotency is delegated to FileSystem impl"
    );
}

// ===== TP-AS*: no-error API 表面 =====

/// spec.md#tp-api-shape TP-AS1 — execute シグネチャに Result / Option を含まない (type-level)
///
/// 注意: 「panic-free」は型レベルでは検証できない。本 TP のスコープは戻り型のみ。
/// Tauri-boundary (commands.rs) の panic-free は infrastructure 層の責務で扱う (spec.md#impl-tauri)。
#[test]
fn tp_as1_execute_returns_settings_not_result_or_option() {
    // 関数ポインタ型として束縛できれば、戻り値が `Settings` 1 個であることが
    // compile-time に保証される。Result<Settings, _> / Option<Settings> では型不一致。
    let _: fn(&LoadSettingsUseCase<RcFs, RcOs>, LoadSettingsCommand) -> Settings =
        LoadSettingsUseCase::execute;
}

/// spec.md#tp-api-shape TP-AS2 — 任意失敗注入時も戻り値は常に有効な Settings (C-LS1 network test)
#[test]
fn tp_as2_settings_returned_under_combined_failures() {
    // 全 io path が壊れた状態: parse 失敗 + ensure_dir 失敗
    let (uc, fs, _) = rig(Some("broken {{ json"));
    fs.fail_ensure(io::ErrorKind::Other);

    let settings = uc.execute(LoadSettingsCommand {
        config_path: config_path(),
    });

    assert!(matches!(settings.theme(), Theme::System));
    assert_eq!(settings.sort_preference().field(), SortField::CreatedAt);
    assert!(settings.storage_dir().as_path().is_absolute());
}

// ===== TP-I1 (property): storage_dir は常に absolute (I-S1) =====

proptest! {
    /// spec.md#tp-invariants TP-I1 — 任意入力に対し settings.storage_dir().is_absolute() (I-S1)
    #[test]
    fn tp_i1_storage_dir_is_always_absolute(content in proptest::option::of(".*")) {
        let (uc, _, _) = rig(content.as_deref());
        let settings = uc.execute(LoadSettingsCommand { config_path: config_path() });
        prop_assert!(
            settings.storage_dir().as_path().is_absolute(),
            "I-S1: storage_dir must be absolute for any input ({:?})",
            content
        );
    }
}

