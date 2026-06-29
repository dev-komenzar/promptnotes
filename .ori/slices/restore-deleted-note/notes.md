# restore-deleted-note — Implementation notes

## Pass 2 LOW follow-up triage (issue ori-hjz)

Pass 2 review (`review.md`) で指摘された LOW 3 件 + Pass 1 LOW 残件の triage 結果。
対応は PR (branch `followup/restore-deleted-note`) に含まれる。

### Pass 2 LOW (3 件 — 全て対応)

| ID | 内容 | 対応 |
|---|---|---|
| LOW-A | `tp_re1` を `['find','trash','load']` order assert に upgrade (MED-7 substrate 活用) | `tests.rs` tp_re1 に `assert_eq!(observed, vec!["find","trash","load"])` 追加 |
| LOW-B | `FakeTrash::restore_from_trash` も常時 log 化 (MED-6/7 と同型) | `tests.rs` FakeTrash で `order_log.push("trash")` を failure check 前に移動。ついでに tp_tr1 にも `vec!["find","trash"]` order assert 追加で対称性クローズ |
| LOW-C | `UnimplementedTrash`/`UnimplementedUndo` + `NoOpBus` の prod wiring follow-up 同梱 track | bd issue として起票 (Pass 1 LOW [12] と同一 issue で cover) |

### Pass 1 LOW 残件 triage

review.md の Pass 1 LOW [2][3][9][10][11][12][13][14] のうち [11] は MED-1 fix で解消済み。
残 7 件の triage:

| review ID | 内容 | triage 決定 | 根拠 |
|---|---|---|---|
| [2] | `ErrorKind::NotFound` for `Ok(None)` collapse | **accepted** (no action) | `oq-read-error-ok-none-policy` で受容済。tp_re2 が NotFound を pin 済み |
| [3] | validation.md s5/s7 が同一 hash `5294b0c32f1b` を共有 | **bd issue 起票** | cross-slice concern。`/ori-sync` で section level hashing を支持するか調査が必要 (ori tooling 側の課題) |
| [9] | `oq-duplicate-deleted-note-by-id` の pinning test 欠落 | **対応済み** | `tp_oq1_duplicate_note_id_first_match_semantics` を `#[ignore]` test として追加。first-match (Vec front) 挙動を文書化し `--ignored` で silent regression 検出可能に |
| [10] | `make_deleted` が production aggregate command 使用 | **accepted** (no action) | review でも OK 判断。delete-note H-1 review fix と同じ convention |
| [12] | `NoOpBus` が prod wiring で event を握り潰し | **bd issue 起票** | LOW-C と同一 issue で cover (delete-note + restore-deleted-note 両方の prod wiring) |
| [13] | `TrashService` trait に `restore_from_trash` を追加したことで test double が `unreachable!()` 強制 | **accepted** (no action) | `oq-trash-service-extension` で文書化済み。SRP より UI 契約優位を選択した design decision |
| [14] | `find_by_id` + `remove_by_id` の 2 step vs `take_by_id` 1 step | **accepted** (no action) | spec I-RDN4 が「leave on stack」semantics を選択。rollback-on-failure より現在の 2 step が正しい。OQ で trade-off 文書化済み |

### 追加した test の実行方法

`tp_oq1` は `#[ignore]` なので通常の `cargo test` では skipped。明示実行で現行挙動を確認:

```bash
cargo test restore_deleted_note -- --ignored
```

### spy substrate 統一状態 (LOW-B 適用後)

3 つの test double が全て「常時 log 化」で統一:

| spy | log token | log 位置 |
|---|---|---|
| `FakeUndo::find_by_id` | `"find"` | failure (None) path 含む全 path で push (MED-6) |
| `FakeRepo::load_by_id` | `"load"` | failure (Err) path 含む全 path で push (MED-7) |
| `FakeTrash::restore_from_trash` | `"trash"` | failure (Err) path 含む全 path で push (LOW-B) |

これにより failure path の副作用順序も order log で pin 可能になった。
