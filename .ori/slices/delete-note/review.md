# delete-note slice review

> Adversarial review (fresh-context spawn). Single pass per `/ori-flow` phase 6.

## Pass 1

Reviewer: ori-reviewer (Claude Opus 4.7 1M, fresh context)
Date: 2026-06-26
Scope: `.ori/slices/delete-note/spec.md` + impl under `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/`

### Findings

#### [1] spec 整合性 / [4] 副作用の境界 — HIGH

**[H-1] `Note::delete_to_trash()` aggregate command がバイパスされている**

- ファイル: `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/application.rs:60-66`
- spec.md#impl-related-slices 行 162:
  > `Note::delete_to_trash()` aggregate command が未実装なら phase 4 で追加（domain/aggregates.md#note-aggregate-commands に既に定義済み）
- `.ori/domain/aggregates.md` 行 83-86 は `Note::delete_to_trash(self) -> DeletedNote` を **公開 commands** として宣言している
- 実装は `load_by_id` の戻り値を即座に `.ok().flatten().ok_or_else(...)?;` で捨てており、`Note` aggregate 上で `delete_to_trash` を呼ぶ経路を持たない。`DeletedNote` は application 層で直接構築されている
- 影響:
  - aggregate boundary を回避して VO を application 層が組み立てるため、**「Note Aggregate の operation の戻り値」(aggregates.md#notes-undo) という設計契約が impl で再現されていない**
  - `apps/.../shared/types/note.rs` には `delete_to_trash` メソッドが存在しない (確認済)
  - 将来 `delete_to_trash` に invariant (例: 削除済み Note の二重削除禁止、削除時刻記録) を追加した時、本 slice からは強制できない構造になっている
- 推奨対応:
  1. `Note::delete_to_trash(self) -> DeletedNote` を `note_capture/shared/types/note.rs` に実装（spec が「未実装なら phase 4 で追加」と明示している通り）
  2. application 層は `let deleted = note.delete_to_trash();` を経由して `DeletedNote` を得る
  3. `tests.rs` に「`Note::delete_to_trash` の戻り値と `UndoStack::push` 引数が同一」を pin する単体テスト (TP-DS1 を強化)

#### [1] spec 整合性 / [6] テスト trace — MED

**[M-1] I-DN7「`DeletedNote.id` は load した Note の `id` と一致」が impl/test 両方で確認されていない**

- ファイル: `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/application.rs:84-87` および `tests.rs:542-569`
- spec.md#invariants-slice-specific 行 85-86:
  > Undo スタックに push する `DeletedNote` の `id` は **load した Note の `id`** と一致し、`original_path` は I-DN1 で導出した path と byte 一致する
- impl は `DeletedNote { id: cmd.note_id.clone(), ... }` と input から直接組み立てている。load した Note の id は読み取られない
- test TP-DS1 (`tp_ds1_deleted_note_id_and_path_match_input_and_storage_dir`) は seed Note の id と cmd.note_id が一致する fixture を使うため、「load Note の id」と「cmd の id」の区別を観測できない
- 影響: misbehaving repo (例: 異 id の Note を返す stub) を仮定した時、spec 文言と impl が乖離。aggregate 経由 (H-1) で組み立てれば自動的に解消される
- 推奨対応: H-1 を採用すれば自然解決。仮に H-1 を採用しない場合は TP-DS1 に「seed Note の id ≠ cmd.note_id」を fail させる guardrail test を追加し、I-DN7 を spec 側で「cmd.note_id と一致」に書き直す (どちらが正かは upstream 提案で決める)

#### [4] 副作用の境界 — MED

**[M-2] `SettingsReader::storage_dir` と `NoteRepository::storage_dir` が同じ値を二重 source で持つ**

- ファイル: `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/ports.rs:36-38`, `apps/.../shared/ports.rs:10`
- 既存 slice (auto_save_note, create_note, copy_note_body) は `NoteRepository::storage_dir()` を経由して path を解決している (`auto_save_note/application.rs:94` 等)
- 本 slice のみ新規 `SettingsReader` port を追加。spec.md#impl-ports 行 145 が要求しているため impl は spec 通りだが、production 配線時に両 port が指す path が乖離するリスクが構造化されている (本 slice の test では同じ STORAGE_DIR 文字列を両方に渡しているため検知不能)
- 影響: phase 7 finalize で `commands.rs` を書く際、2 つの異なる `Path` を注入できてしまう。delete された file の path と save 時の path がずれると I-DN1 が破綻
- 推奨対応:
  - **案 A (impl 側)**: `SettingsReader` port を廃止し、`NoteRepository::storage_dir()` を使う (既存 slice と一貫)
  - **案 B (spec 側)**: 両 port が同一インスタンスを共有する制約を spec/architecture.md に追記
  - **案 C (test 強化)**: production wiring に `assert_eq!(repo.storage_dir(), settings.storage_dir())` の wiring-time check を追加

#### [5] edge case — MED

**[M-3] `Note::delete_to_trash` が consume するという aggregates.md の semantics が test で pin されていない**

- ファイル: `tests.rs` 全体
- aggregates.md#note-aggregate-commands 行 83 は `Note::delete_to_trash(self) -> DeletedNote` (consume 形) と明示している。これは I-N7「削除された Note の identity は DeletedNote スタックに push され、対応する Toast の有効期間中のみ復元可能」を type-level で保証する重要な aggregate 契約
- 現 impl は loaded Note を即破棄するため consume 形は不要であり、tests も「同一 id の Note を再度 load して 2 回 delete を試す」のような double-delete reject ケースを持たない
- 影響: 「同じ note を 2 回 delete したら何が起きるか」が pin されない。現実装では 2 回目も成功して Undo スタックに重複 push される。これが domain として正しいかは spec 未定義
- 推奨対応: H-1 を採用しつつ、TP-NF / TP-TE と並列に「double-delete 観点」テストを追加するか、spec で明示的に「double-delete は port (NoteRepository) 側で `Ok(None)` が返るのでこの slice の単体テスト境界外」と pin

#### [5] edge case — MED

**[M-4] TrashError 後の `original_path` が test で「I-DN1 通り」確認されていない (TP-TE2)**

- ファイル: `tests.rs:421-441`
- spec.md#tp-trash-err:
  > `DeleteNoteError::TrashError { path, cause }` が返り、(...) **path は I-DN1 通り `storage_dir / <id>.md` と一致**
- TP-TE1 (`tp_te1_trash_error_propagates_and_blocks_push_event`) は `path == expected` を assert しているが、TP-TE2 (`tp_te2_trash_io_error_preserves_cause`) は `cause` のみ確認し path を見ていない
- 推奨対応: TP-TE2 にも `assert_eq!(path, expected)` を追加して I-DN1 を Io variant でも pin

#### [6] テスト ↔ spec trace — LOW

**[L-1] TP-TO1 が「構造的に確認」を behavior assert で代用している**

- ファイル: `tests.rs:574-590`
- spec.md#tp-trash-only:
  > **本 slice の impl コードに `std::fs::remove_file` / `std::fs::remove_dir_all` 等の unlink API への直接依存が出現しないことを構造的に確認 (I-DN2)**
- 現 test は `repo.write_count() == 0` のみ確認。impl ソース上に `fs::remove_file` が無いことは grep で確認 (comment 中のみ) したが、自動化されていない
- 影響: 将来 contributor が `std::fs::remove_file` を直接呼ぶ regression を test では検知できない
- 推奨対応: build.rs もしくは `tests/grep_forbidden.rs` で slice ソース文字列を読み `fs::remove_` を禁止する static check を追加。または clippy custom lint。最低限 spec で「構造確認は CI で行う」と注記

#### [3] DDD 規約遵守 — LOW

**[L-2] DeletedNote が aggregate boundary を跨ぐ VO として slice 内で定義されている**

- ファイル: `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/domain.rs:14-18`
- aggregates.md#notes-undo は DeletedNote を Note Aggregate の「operation の戻り値」と位置付ける。restore-deleted-note slice と共有される
- 現状 `DeletedNote` 型は delete_note slice の `domain.rs` に局在化。restore-deleted-note slice が出現した時、型を share kernel 化する re-home が必要になる
- 影響: 単純な future refactor。今 phase で is fix する必要はないが、phase 7 finalize / restore slice 着手時に shared/types/ へ移動する TODO を残すべき
- 推奨対応: `apps/.../shared/types/deleted_note.rs` への移動を follow-up issue 化。あるいは `pub use` で shared から re-export する暫定経路を作る

#### [3] DDD 規約遵守 — LOW

**[L-3] `Result` 型を throw で代用していない (確認のみ、指摘なし)**

- impl 全体で `panic!` / `unwrap` の production code 使用なし。`expect` は test fixture (`tests.rs:259, 269` 等) のみ
- `?` propagation 正常使用。Railway-Oriented Programming 準拠

#### [7] 冗長性 — LOW

**[L-4] `RcRepo` / `RcTrash` / `RcUndo` / `RcBus` wrapper の重複**

- ファイル: `tests.rs:76-87, 124-129, 157-162, 222-227`
- 4 つの port 全てが `Rc<Fake*>` を `trait` 実装にラップする wrapper struct を持つ。各 wrapper は単純 delegate
- 影響: 機械的な boilerplate。`impl<T: NoteRepository + ?Sized> NoteRepository for Rc<T>` のような blanket impl を `shared/ports.rs` に追加すれば全 slice の test boilerplate を削減できる
- 推奨対応: shared/ports.rs に `impl<T: NoteRepository> NoteRepository for Rc<T>` (および Trash/Undo/Bus) を導入する refactor を follow-up 化。本 slice の review pass 1 では非ブロッキング

---

### 良かった点 (positive observations)

- 副作用順序 `trash → push → event` を OrderLog 共有で観測する TP-SO1 設計は I-DN5 を厳密に pin している
- I-DN6 collapse の意図的選択 (copy-note-body I-CNB5 と同型) を TP-NF3 で明示的に pin
- error variant に `#[source]` を付与し thiserror chain を保つ (`domain.rs:28`)
- 各 test に `spec.md#tp-*` の anchor 注釈があり trace が機械的に追える
- `NoteDeletedToTrash` event variant の追加に対し既存 102 lib tests + 13 slice tests GREEN を保持

---

### Disposition

**NEEDS_FIX**

理由:
1. **H-1 (Note::delete_to_trash bypass)** は spec impl-notes が「未実装なら phase 4 で追加」と明示している aggregate command を実装していない構造的な spec 不整合であり、I-DN7 (M-1) と aggregate boundary semantics (M-3) の根本原因
2. M-1 / M-2 / M-3 / M-4 は H-1 解消で 3 件 (M-1, M-3 部分, テスト微修正) が自然に解決されるか不要化される
3. M-2 は spec 自体に二重 source の余地があるため、impl fix と spec 提案 (案 A or 案 B) のいずれかを選ぶ要

優先順位:
1. **必須 (NEEDS_FIX 解消条件)**: H-1 (Note::delete_to_trash aggregate 実装) + M-4 (TP-TE2 path assert)
2. **強推奨**: M-1 (TP-DS1 強化 — H-1 後), M-2 (案 A: SettingsReader 廃止 or 案 B: spec 更新)
3. **follow-up (本 review pass で blocking しない)**: L-1 (構造 check 自動化), L-2 (DeletedNote re-home), L-4 (Rc wrapper blanket impl), M-3 (double-delete 観点)

H-1 を解消できれば残り MED の半分は連鎖解消する。最少修正は **Note::delete_to_trash 実装 + 既存 application.rs を `note.delete_to_trash()` 経由に書き直す + TP-DS1 で seed Note の id を観測 + TP-TE2 に path assert 追加** の 4 点。

---

## Pass 2

Reviewer: ori-reviewer (Claude Opus 4.7 1M, fresh context)
Date: 2026-06-26
Scope: Pass 1 patch 後の差分検証
- `apps/promptnotes/src-tauri/src/note_capture/shared/types/deleted_note.rs` (新規)
- `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs` (`delete_to_trash` 追加)
- `apps/promptnotes/src-tauri/src/note_capture/shared/types/mod.rs` (re-export 追加)
- `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/{domain.rs, ports.rs, application.rs, tests.rs, mod.rs}`

### Pass 1 finding 解消確認

**[H-1] aggregate bypass — 解消 (PASS)**

- `shared/types/note.rs:100-102` で `Note::delete_to_trash(self, original_path: PathBuf) -> DeletedNote` を実装。`self` 消費形で aggregates.md#note-aggregate-commands 行 83 と一致
- `shared/types/deleted_note.rs:20-22` で `DeletedNote::new` が `pub(crate)` に制限され、構造的に「aggregate のみが DeletedNote を mint できる」契約を型システムで担保
- `application.rs:90` で `let deleted = note.delete_to_trash(original_path.clone());` 経由で構築。grep 確認 (`DeletedNote::new` の呼出箇所は `shared/types/note.rs:101` 唯一) で他 slice / 他 path からの bypass 無し
- 結果: aggregate boundary が impl と型レベルの両方で再現された

**[M-1] I-DN7 「load Note の id と一致」が確認可能 — 解消 (PASS)**

- `DeletedNote::new` が `pub(crate)` で限定されたため、`Note::delete_to_trash(self, ...)` のみが id source。`self.id` を VO に転記するため「loaded Note の id == DeletedNote.id」が型システムで担保 (`note.rs:101`)
- `tests.rs:526-562` TP-DS1 の docstring (528-531 行) で「型レベル担保 + 表面値確認」の役割分担が明示されている
- 結果: 表面値テスト + 型レベル不変が二段で pin される

**[M-2] SettingsReader 重複 port — 解消 (PASS)**

- `ports.rs:1-32` から `SettingsReader` trait を削除。残るのは `TrashService` / `UndoStack` のみ
- `application.rs:74-77` は `self.repo.storage_dir().join(format!("{}.md", cmd.note_id.as_str()))` で `NoteRepository::storage_dir()` を再利用 (auto-save-note slice と一貫)
- `mod.rs:10` の re-export も `pub use ports::{TrashErrorKind, TrashService, UndoStack};` で SettingsReader 不在
- 結果: 二重 source が消滅。phase 7 finalize で wiring 時の path 乖離リスク無し

**[M-3] double-delete が未定義 — 解消 (PASS)**

- `Note::delete_to_trash(self, ...)` が `self` 消費するため、同一 in-memory Note インスタンスに対する 2 回目の呼出はコンパイル時不可能 (move-after-move)
- `application.rs:23-27` の docstring が「2 回目の `execute()` は load → None → NoteNotFound で短絡」と明示。spec.md#tp-not-found と整合
- 結果: in-memory double-delete は型レベル禁止、persistence layer の double-delete は既存 spec で網羅済

**[M-4] TP-TE2 path assert 欠如 — 解消 (PASS)**

- `tests.rs:407-430` `tp_te2_trash_io_error_preserves_cause_and_path` が `assert_eq!(path, expected, "I-DN1: error carries derived path")` を 425 行で追加
- I-DN1 が PermissionDenied (TP-TE1) / Io (TP-TE2) の両方で pin された
- 結果: spec.md#tp-trash-err の path 要求が両 variant でカバー

### Regression check

1. **shared/types/ 変更による他 slice への影響**: `Note` には `delete_to_trash` メソッドが追加されただけで、既存 `create` / `from_persisted` / `edit_body` / getter / `body_for_clipboard` の signature 変更無し。`DeletedNote` は新規型なので破壊なし
2. **全 lib tests**: 102/102 GREEN (`cargo test --lib`)
3. **clippy**: `cargo clippy --lib --all-targets -- -D warnings` clean
4. **DeletedNote 構築箇所の grep**: 唯一の構築箇所が `note.rs:101` の `DeletedNote::new(self.id, original_path)` で、aggregate boundary 経由が型レベルで強制されている (test fixture も `note_a.delete_to_trash(...)` 経由 — `tests.rs:502`)
5. **event variant**: `shared/events.rs:20-24` の `NoteDeletedToTrash` payload は Pass 1 から不変。既存 subscriber への影響なし

### 残 LOW 指摘の現状

- **[L-1] 構造的 unlink ガード**: `fs::remove_*` の slice 内出現は port 定義の doc comment (`ports.rs:20`) のみ。impl 経路には無し。自動化は未導入だが Pass 1 から悪化していない (follow-up issue 化が妥当)
- **[L-2] DeletedNote 配置**: Pass 2 で `shared/types/deleted_note.rs` へ移動済み。これにより L-2 は **解消** (restore-deleted-note slice 着手時の re-home 不要に)
- **[L-4] Rc blanket impl**: 4 つの `RcRepo` / `RcTrash` / `RcUndo` / `RcBus` wrapper は依然 `tests.rs:79-90, 127-132, 160-165, 209-214` に boilerplate として残る。Pass 1 から悪化なし (follow-up issue 化が妥当)

### 新規 finding

なし (HIGH / MED 共に 0 件)。

### 良かった点 (Pass 2)

- `DeletedNote::new` の `pub(crate)` 化により、I-DN7 (id 一致) と aggregate boundary semantics が **型システムで** 担保される設計に昇格 (Pass 1 では behavior assert に依存していた)
- `application.rs:7-27` の docstring が pipeline 6 step を順序付きで列挙し、各 step が引く invariant (I-DN1〜I-DN8 + I-N7) を逐一明示。spec ↔ impl の trace が機械的に追える
- TP-SA1 (`tests.rs:494-522`) で seed `DeletedNote(A)` の construction を `note_a.delete_to_trash(...)` 経由に統一し、test fixture も production 経路と同じ単一 construction site を共有
- M-2 解消で port 数が 6 → 5 に減り、phase 7 finalize の wiring 複雑性が一段下がった

### Disposition

**PASS**

理由:
1. Pass 1 で blocking だった H-1 / M-1 / M-2 / M-3 / M-4 が全て解消し、根拠 (型レベル担保 / port 削除 / docstring 明示 / assert 追加) を impl 上で確認
2. 102 lib tests GREEN + clippy `-D warnings` clean、regression なし
3. shared/types/ への変更は additive で他 slice (`auto-save-note` / `create-note` / `copy-note-body` / `assign-tag` / `flush-note`) の既存契約に影響なし
4. 残 LOW (L-1 / L-4) は本 review の blocking 対象外。L-2 は Pass 2 で解消
5. 新規 HIGH / MED finding 無し
