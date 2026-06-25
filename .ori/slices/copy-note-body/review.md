# Review: copy-note-body {#review-copy-note-body}

## Pass 1 {#pass-1}

Reviewer: claude-opus-4-7 (capability=reasoning, fresh context)
Date: 2026-06-25
Slice phase: 6 (review of phase 4 impl + phase 5 refactor)
Test status (verified locally): `cargo test --lib note_capture::slices::copy_note_body` = 11 / 11 passing

### Findings {#pass-1-findings}

#### HIGH

- **HIGH-1** `application.rs:32-41` ↔ `spec.md#io-errors` / `tests.rs`: `repo.load_by_id` の `io::Err` を `NoteNotFound` に collapse する設計判断が **テストで一切検証されていない**。`tests.rs:31` で `fail_load_with: Cell<Option<io::ErrorKind>>` というフックを準備しているのに、それを使う test が 1 件も存在せず、`grep` で確認すると seed / read で `fail_load_with.take()` を経由するのは `Cell::new(None)` の初期化と空 take だけ。impl-green の close notes は「これは意図的選択。spec を直すか impl を直すかは phase 6 で revisit」と明記しているが、**意図的選択を主張するなら invariant として spec に書き、test で pin する**のが ori 規約の前提。現状は「LoadError は隠す」が暗黙の仕様になっており、将来 disk read 失敗を debug したいエンジニアが「note が消えたように見える」謎挙動に当たる。
  - 推奨: 以下のどちらか。
    1. **spec 修正路線（impl 現状維持）**: spec.md#io-errors に「`NoteRepository::load_by_id` の `io::Err` は `NoteNotFound` に collapse する（情報損失するが、user-observable には note 不在と区別できないため契約として吸収）」を追記し、`spec.md#invariants-slice-specific` に I-CNB5 として明文化。同時に test `tp_nf3_repo_io_error_collapses_to_note_not_found` を追加して seed しないで `repo.fail_load_with.set(Some(io::ErrorKind::PermissionDenied))` を仕掛けたうえで `NoteNotFound` を assert、かつ `clipboard.write_count() == 0` を assert（I-CNB3 強化）。
    2. **impl 修正路線**: `CopyNoteBodyError` に `LoadError { cause: io::ErrorKind }` を追加し spec.md#io-errors を 3 variant に拡張。`application.rs` の `.ok().flatten().ok_or(...)` を `match` に分解。`tests.rs` の `fail_load_with` を使う test を追加。
  - 差し戻し先: 路線 1 なら **propose**（domain workflow `Errors` の修正 proposal）+ test-red 1 件追加。路線 2 なら **test-red → impl-green** の 2 phase 巡回。**推奨は路線 1**（spec の error variant を 2 に絞った decision に backward-compatible で、user-observable な振る舞いも spec の現状記述と矛盾しない）。

#### MED

- **MED-1** `tests.rs:31, 40, 62-64`: `fail_load_with` インフラが死蔵されている。HIGH-1 の決定方針が決まれば必要な test を追加するか、未使用ならフィールドごと削除する。死コードを残すと「ここで何を testing しているのか分からない」レビュー負荷を後続セッションに転嫁する。
  - 推奨: HIGH-1 の解消とセットで処理。propose 路線なら test 追加で die hookup を生かす。

- **MED-2** `tests.rs:347-351` TP-BC1 docstring: 「`Note::body_for_clipboard()` を導入したら phase 4 / refactor でこの assertion を `note.body_for_clipboard()` 経由に差し替える」というコメントが残っている。Phase 5 (refactor) は終了済みで、`Note::body_for_clipboard()` も `shared/types/note.rs:82-84` に導入済み。**コメントが古いまま**で、結局 「seed body と byte-for-byte 一致」のみを assert する形になっており、I-CNB1 の「`body_for_clipboard()` 経由」契約は **テストでは pin されていない**（spec.md#tp-uses-body-for-clipboard が「TBD: compile-time / test-time の選択に応じて調整」と書いていた論点が解決されないまま phase 5 が完了している）。
  - 推奨: docstring を refactor 後の事実に書き換える。さらに「`body_for_clipboard()` 経由」を本当に pin したいなら、`tp_bc2_clipboard_string_equals_body_for_clipboard_return`（`note.body_for_clipboard()` の戻り値と `clipboard.last_written()` が一致することを assert）を追加。`tp_bc1` の「seed body」と「`body_for_clipboard()` 戻り値」は今は同値だが、将来 `body_for_clipboard()` に normalization (trailing newline 付与等) が入った時に **TP-BC1 だけだと regression を検出できない**。
  - 差し戻し先: **refactor**（コメント修正）+ **test-red** で TP-BC2 追加（任意、優先度は HIGH-1 より下）。

- **MED-3** `ports.rs:6-10` `ClipboardErrorKind`: variant が `Unavailable` と `Io(String)` の 2 種類だが、spec.md#io-errors は variant 集合を enumerate していない（「`ClipboardErrorKind::*` を返す」とだけ）。**spec が variant 集合を契約していない**ため、phase 7 で Tauri adapter を実装した際に variant が足りない / 命名が合わないことが発覚するリスク。同 file のコメントは「phase 4 / refactor may widen it」と明示しているが、widen 時に spec / test / impl の 3 箇所同時更新が必要になる。
  - 推奨: spec.md#io-errors に variant 集合（`Unavailable | Io(String)`）の明文化 + 「拡張時は spec と test を同時更新」の運用注記。**現状では blocker ではない**が finalize 時に follow-up issue として残すべき。
  - 差し戻し先: **propose**（spec/domain への error 列挙の補強）または finalize 時 follow-up。

- **MED-4** `domain.rs:14-15` `ClipboardError { cause }`: error の transparent forwarding として `#[source]` annotation が無い。`thiserror::Error` の `cause: ClipboardErrorKind` は表示こそ `{cause:?}` で出るが、`std::error::Error::source()` で chain を辿れない。production で error log を取った時に root cause が `Debug` 表示でしか復元できず、構造化ログとの相性が悪い。
  - 推奨: `ClipboardErrorKind` に `std::error::Error` を実装し（`thiserror` で OK）、`CopyNoteBodyError::ClipboardError { #[source] cause: ClipboardErrorKind }` にする。**現状 GREEN を破らない**範囲の修正。
  - 差し戻し先: **refactor**。

#### LOW

- **LOW-1** `tests.rs:69-80, 111-116` `RcRepo` / `RcClipboard` wrapper struct: `auto_save_note` slice の test と同じパターンだが、共有してない。重複コスト自体は小さい (15 行) ので無理に shared module 化する必要は無いが、slice 間で test rig 構造が**揺れる**と「ある slice では Rc + Wrapper、別の slice では Arc + Wrapper」という divergence を生む。
  - 推奨: backlog / accept。3 つ目の slice (clipboard 系) が出てきた時に共通化を検討。

- **LOW-2** `application.rs:36-41`: `.ok().flatten().ok_or(...)` の 3 段 chain は機能するが、`load_by_id` の戻り値 `io::Result<Option<Note>>` を扁平化するために 2 つの failure mode (`Err(_)` と `Ok(None)`) を同じ variant に潰している。Rust の `match` で書いた方が「2 失敗を 1 variant に collapse している」という意図が読み手に届く。
  - 推奨: HIGH-1 の決定とセットで形を見直す。

- **LOW-3** `ports.rs:8` `ClipboardErrorKind` 名: 「Kind」だが variant を持つ enum なので慣例上 OK（cf. `io::ErrorKind`）。一方 `domain.rs:15` の `cause: ClipboardErrorKind` field 名は、enum 自体を cause として持つので「kind を cause と呼ぶ」のは命名 mismatch。`source` または `kind` の方が素直。
  - 推奨: backlog / accept。

- **LOW-4** `manifest.yaml:8`: `implementation.generates` に TS 側 (`apps/promptnotes/src/lib/note-capture/slices/copy-note-body/`) を含んでいるが、phase 4/5 で**生成されていない**（TBD-1: Tauri plugin 採用が phase 7 持ち越し）。manifest が「生成予定」と「生成済み」を区別しないため、`/ori-sync` 等で drift を見落とす可能性。
  - 推奨: finalize 時の follow-up issue で TS 側生成タスクを明示。

### Disposition {#pass-1-disposition}

- HIGH-1 → **propose**（推奨: spec に I-CNB5 として LoadError collapse を明文化）+ **test-red**（`tp_nf3` 追加）
- MED-1 → HIGH-1 とセット
- MED-2 → **refactor**（docstring 更新 + TP-BC2 追加検討）
- MED-3 → **propose** or finalize follow-up
- MED-4 → **refactor**（`#[source]` 付与）
- LOW-1〜4 → backlog or accept

### 総合判定 {#pass-1-overall}

**NEEDS_FIX** — 一番優先する差し戻し先は **propose**（HIGH-1: LoadError collapse の spec 反映 + 対応 test 追加）。

**理由**:
1. impl-green の close notes 自身が「phase 6 で revisit」と保留した設計判断 (LoadError collapse) が、spec にも test にも残らないまま phase 6 を通過しようとしている。これは spec/impl drift の典型ケースで、ori workflow の SSoT 原則に直接抵触する。
2. テスト infra (`fail_load_with`) を仕込みながら 1 件も使わない状態は、後続セッションへの「埋伏地雷」になる。fresh-context レビュアが見ると「なぜこのフックがある？」が即座に疑問符化する。
3. 一方、impl のレイヤ配置・port 設計・I-CNB1〜I-CNB4 の構造的 enforce・refactor 後のコード品質は概ね適切で、HIGH 指摘は **1 件のみ**。HIGH-1 を解消すれば PASS 相当。

**production readiness 観点**:
- `commands.rs` 不在は spec.md#impl-layout で予告済み（TBD-1）なので drift ではないが、finalize 時に follow-up issue として **必須**で beads 化する。現状の slice は production 経路から到達不能であり、unit test green は production correctness を意味しない（reviewer 観点 7: production readiness）。

## Pass 2 {#pass-2}

Reviewer: claude-opus-4-7 (capability=reasoning, fresh context)
Date: 2026-06-25
Slice phase: 6 (Pass 2 — final round, no Pass 3)
Test status (verified locally): `cargo test --lib copy_note_body` = 13 / 13 passing; `cargo clippy --lib -- -D warnings` = clean

### Pass 1 findings 解消検証 {#pass-2-resolution-check}

#### HIGH-1 — RESOLVED ✅

- spec.md#invariants-slice-specific I-CNB5 (line 72) が substantive に追記済み。trade-off（永続化層 debug 損失）と escalation path（diagnostic 要請が出たら 3 variant に拡張、spec/test 同時更新）が明文化されている。
- spec.md#io-errors (line 53-57) が `NoteNotFound` 説明に「`io::Err` 帰着は I-CNB5 により本 variant へ collapse」を組み込み、`#tp-repo-io-err-collapse` への cross-reference も存在。
- tests.rs:269-293 `tp_nf3_repo_io_error_collapses_to_note_not_found_with_no_clipboard_write` が `fail_load_with.set(Some(io::ErrorKind::PermissionDenied))` を仕掛けたうえで `NoteNotFound { id }` を assert、追加で `clipboard.write_count() == 0` で I-CNB3 + I-CNB5 の整合契約を pin。docstring に「impl が将来 `LoadError` を導入したらこの test は失敗 → spec を同時更新」と SSoT 紐付けを明示。
- 「intent → spec → test」chain が閉じ、impl-green close notes で保留された設計判断が永続化された。

#### MED-1 — RESOLVED ✅

- `fail_load_with` infra が TP-NF3 で実利用された。死蔵されていないことを `cargo test` の pass で確認。`fail_next_with` / `fail_load_with` ともに使われている。

#### MED-2 — RESOLVED ✅

- tests.rs:379-384 TP-BC1 docstring が refactor 後の事実に書き換えられ、「phase 4 / refactor で差し替える」相当の stale 表現は除去済み。新 docstring は「TP-BC1 = 表面契約、TP-BC2 = 別軸 pin」と分離を明示。
- TP-BC2 (tests.rs:408-422) が `seed.body_for_clipboard()` を seed 投入前に capture し（move 順序正しい）、clipboard 内容と byte-for-byte 一致を assert。現状 `body_for_clipboard()` は `self.body.as_str().to_string()` のため両者は同値だが、将来 normalization が追加された時の regression を捕捉する **構造的 pin** として機能する（docstring にも明記）。

#### MED-3 — RESOLVED ✅

- spec.md#io-errors (line 55) で `ClipboardErrorKind` を `Unavailable | Io(String)` と最小集合明文化。拡張時に「spec + test 同時更新」運用も併記済み。

#### MED-4 — RESOLVED ✅

- domain.rs:18 `#[source]` 付与済み。`thiserror::Error` derive 経由で `std::error::Error::source()` chain が機能。
- ports.rs:4 `ClipboardErrorKind` も `thiserror::Error` を derive し、variant ごとに `#[error("...")]` で Display を実装。`CopyNoteBodyError::ClipboardError` の Display path (`{cause}`) が `Debug` ではなく Display 経路に切り替わったことも domain.rs:16 で確認。構造化ログ adapter から root cause 到達可能。

### 新規所見 {#pass-2-new-findings}

#### LOW-NEW1 — spec.md line 115 の表現が stale

spec.md#tp-uses-body-for-clipboard 末尾（line 115）に「`tp_bc2`（**追加検討中**）」という表現が残存。TP-BC2 は既に tests.rs:408-422 に実装済みで、「追加検討中」は事実と乖離。

- 推奨: spec.md line 115 の「追加検討中」を削除し、「TP-BC2 は body_for_clipboard 戻り値との byte 一致を assert する」と確定形に書き換える。または finalize 時の sweep に折り込む。
- 重大度: LOW。spec の動作契約は I-CNB1 / I-CNB5 で十分 pin されており、test も実装されているため実害は無い。文書 hygiene 上の指摘。
- 差し戻し先: finalize 時の follow-up（**blocker ではない**）。

#### LOW-NEW2 — TP-BC2 は現時点で TP-BC1 と振る舞いが等価

`body_for_clipboard()` の現実装が `self.body.as_str().to_string()` のため、TP-BC2 の `expected` は seed body と等しく、TP-BC1 と assert 内容が同値。docstring が「regression sensor」と framing しているため設計上の意図は明確だが、現時点では真の orthogonal pin として機能していない（path divergence の sensor は将来 `body_for_clipboard()` が分岐したときに有効化する）。

- 推奨: 受容可。docstring の framing で意図は伝わっている。`body_for_clipboard()` が将来 normalization を追加した時点で TP-BC2 が真に意味を持つ。
- 重大度: LOW（accept）。

#### INFO — 派生文書編集と /ori-sync --force {#pass-2-info-derive}

spec.md frontmatter の `last_derived: 2026-06-25` コメントで「upstream hash 不変（slice 固有 invariant の追加のため）」と addition の性質が記録されている。本来 ori-conventions.md は派生文書直接編集前に `/ori-sync --force` で proposal を生成する規約だが、I-CNB5 / `#tp-repo-io-err-collapse` は **slice-local invariant** の追加であり upstream domain doc に対応 section が無いため proposal 生成対象が無い。実害は無いが、`/ori-finalize` で dirty propagation 確認時に「upstream への propose 候補が無いこと」を明示確認しておくと安全。

- 重大度: INFO。blocker でない。finalize 時の sanity check 項目として残す。

### Pass 2 で確認した非 regression {#pass-2-non-regressions}

- `cargo test --lib copy_note_body` = 13 / 13 passing（TP-NF3 / TP-BC2 含む）
- `cargo clippy --lib -- -D warnings` = clean
- domain.rs / ports.rs / application.rs / tests.rs の change set に新規 dead code 無し
- `fixture_note_with_tags` / `fixture_note` の利用方針は phase 5 と一貫
- I-CNB1〜I-CNB5 の test pin が網羅された（TP-EX1 → I-CNB1、TP-EB1 → I-CNB2、TP-NF2/TP-NF3 → I-CNB3、TP-NE1 → I-CNB4、TP-NF3 → I-CNB5）

### 総合判定 {#pass-2-overall}

**PASS** — slice は phase 7 (finalize) に進める。

**理由**:
1. Pass 1 で挙げた HIGH-1 / MED-1〜4 がすべて substantive に解消された。設計判断 (LoadError collapse) は spec の I-CNB5 + test の TP-NF3 で SSoT が閉じている。
2. test 13 件・clippy clean で regression 無し。`#[source]` 付与で error chain も production grade に整った。
3. 新規所見は LOW（文書 hygiene）と INFO（運用確認）のみで、phase 7 finalize の通常 sweep に折り込める範疇。
4. `commands.rs` 不在と TS slice 未生成は spec.md#impl-layout TBD-1 と manifest.yaml で既に予告済みであり、Pass 1 で「finalize 時 beads 化」と disposition 済み。新規 finding ではない。

**production readiness 観点 (再掲)**:
- 本 slice は依然 production 経路に未接続（`commands.rs` 不在）。unit test green は domain / application 層の correctness のみ保証する。`/ori-finalize` で `commands.rs` 作成 + TS bindings 生成 + manifest.yaml の `generates` 整合の follow-up beads を必ず発行すること。

**Phase 7 finalize での持ち越し項目**:
- spec.md line 115 の「追加検討中」削除（LOW-NEW1）
- `commands.rs` 実装 + Tauri adapter 採用判断（既存 follow-up）
- TS slice generate（manifest.yaml.generates 整合、既存 follow-up）
- upstream domain doc への propose 候補が無いことの sanity check（INFO）
