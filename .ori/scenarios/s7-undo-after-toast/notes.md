# s7-undo-after-toast — Scenario implementation notes

## Event 検証の方針 (NoOpBus 制約) {#event-verification-policy}

`restore_deleted_note` の production 配線も `NoOpBus`
(`restore_deleted_note/commands.rs`) のため、`NoUndoAvailable` 時に
`NoteRestoredFromTrash` が**発行されない**ことは E2E で直接観測できない。
担保は各 slice の unit test（`restore_deleted_note/tests.rs` の I-RDN1）と、
A の `.md` が原パスへ復帰しないこと（= 副作用が起きていない）による間接検証。
E2E は UI + FS 状態変化でカバーする（s5 / s6 と同方針）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| A 削除後のファイル不在（trash へ移動） | E2E (`existsSync` false + `trash/A.md` true) |
| A / B の Toast が 2 件同時表示（置換なし） | E2E (toast count = 2 + 両 `data-toast-id` 存在) |
| A の Toast が TTL 5s で消失（B は残存） | E2E (`waitUntil` で A の `data-toast-id` が 0 件 / B は 1 件) |
| **復元 API が `NoUndoAvailable` で reject** | E2E (`restore_deleted_note` を直接 invoke → `kind=no_undo_available`) |
| A はゴミ箱に残る（原パスに復帰しない） | E2E (`.md` absent + `trash/A.md` present) |
| per-toast 独立性（B の Undo は有効） | E2E (B の toast undo → `.md` 復元 body=`bravo` / B toast 消滅) |
| event 非発行 | slice unit test（E2E 不可・上記） |
| Undo スタックの per-element TTL prune | Rust unit test (`undo_stack.rs::expired_entry_is_pruned_and_not_found` / `expiry_is_per_element`) |

## production 修正 {#production-fix}

初回 E2E は **t2 で RED**（`attempt.ok` が `true` = 期限切れ後も復元が成功）。
原因は application service 側の `InMemoryUndoStack` に TTL が無く、
frontend の Toast timeout（5s）で UI エントリが消えても Rust 側の `DeletedNote` が
残り続けていたこと。これは `delete-note.md#dependencies`
「`UndoStack` は TTL 管理付き、各要素ごとに個別タイマー」/
`restore-deleted-note.md#errors`「Toast 消失後は `NoUndoAvailable`」に違反する
production 側の不足。

- 最小修正: `InMemoryUndoStack` を `Mutex<Vec<Entry>>`（`Entry { deleted, expires_at }`）へ変更し、
  push 時に `Instant::now() + UNDO_TTL (5s)` を付与。`find_by_id` / `remove_by_id` / `push` の
  アクセス時に期限切れエントリを lazy prune する（`prune_expired`）。
- `UndoStack` trait の形は不変（push / find_by_id / remove_by_id）。frontend 変更なし。
- per-element TTL は push 時刻基準で独立 → per-toast 独立性を構造的に維持。
- 回帰確認:
  - Rust unit test: `note_capture::shared::adapters::undo_stack` 4/4 pass。
  - E2E: 修正前 **1 failing**（`Expected: false, Received: true`）→ 修正後 **1 passing (9.9s)**。
  - `cargo clippy --all-targets`: 変更ファイルに警告なし（既存の他ファイル警告のみ）。
  - 備考: `cargo test --lib` の `note_feed::...::tp_f5_last_7_days` /
    `tp_f6_and_composition` は固定日付 (2026-06) × 現在日付 (2026-09) に依存する
    **既存** failure で、本変更とは無関係（未変更モジュール）。
