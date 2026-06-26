# flush-note review {#flush-note-review}

Adversarial review per ori-reviewer agent definition. Fresh-context, no
inheritance from the implementing session. References below are absolute
paths from repo root.

## Pass 1

### [1] spec ↔ impl 整合性 — C-FL1 production race-prevention is voided

[1.1] `apps/promptnotes/src-tauri/src/note_capture/slices/flush_note/commands.rs:36-41`
production composition root wires `NoOpDebounceTimer` (`cancel` is an empty
fn body). spec.md#invariants-slice-specific C-FL1 explicitly states the
purpose of cancel: 「cancel と persist の順序は固定: cancel → load → ... →
persist（cancel 後に AutoSave が並走して重複 write が走る race を防ぐ）」。
With a NoOp at the composition root, the **race prevention contract is
not actually enforced in production**: if a JS-side AutoSave debounce
fires its `auto_save_note` Tauri command between the moment the frontend
dispatches `flush_note` and the moment Rust completes the write, two
concurrent writes can interleave. The pipeline step still runs in the
declared order, but the *guarantee* C-FL1 is meant to provide
(`AutoSave 並走防止`) is delegated entirely to the JS timer being
synchronously cancellable before invoke is awaited — which is not
verifiable from Rust.

This is partially documented in spec.md#impl-debounce-port ("実装は
composition root で frontend と橋渡しする") and the `NoOpDebounceTimer`
comment ("Cancellation is owned by the UI-side timer"), but neither
discloses the consequence: the slice ships without backend-side race
prevention. Tests pass because `FakeTimer` simulates the contract; the
Rust production path does not.

Recommendation: file as a known limitation or OQ. Either (a) add a
`#oq-debounce-cancel-composition` open question to spec.md acknowledging
the gap and noting that until a Rust-resident debounce timer is wired
(e.g., a `tokio::sync::Notify` shared with the AutoSave command), the
contract is upheld only by the JS layer; or (b) move C-FL1 wording from
hard invariant to "composition root delegates to UI-side timer; backend
trusts cancel-before-invoke". Without this, C-FL1 is currently false in
production.

### [2] derives_from の網羅

[2.1] S13 (`domain/validation.md#s13-quit-flush`) requires "quit シグナル
受信 → 全 EDITING ブロックを Flush → `Note::edit_body` を A, B, C の順に
同期実行 → quit 完了まで永続化を待つ". The slice ships **no Rust-side
quit orchestration** — `apps/promptnotes/src-tauri/src/lib.rs:5-30` does
not install `on_window_event(WindowEvent::CloseRequested)` and never
gates `app.exit(0)`. spec.md#impl-quit-orchestration acknowledges this is
deferred to frontend, and spec.md#out-of-scope lists "S13 の「複数
EDITING Note の順次処理」 — Tauri 起動部の責務". This is internally
consistent but worth flagging: the slice's derives_from declares
dependency on `validation.md#s13-quit-flush`, yet the only S13-related
artifact is the type-level pin TP-S13-2 (which proves single-command
responsibility, the *opposite* of S13). The actual S13 satisfaction is
deferred. Recommend either (a) removing `s13-quit-flush` from
manifest.derives_from and noting it as cross-slice integration, or (b)
filing a follow-up issue for the Tauri-side `on_window_event` wiring
(currently not in beads).

[2.2] Other derives_from items (`workflows/flush-note.md#flush-note`,
`aggregates.md#note-aggregate`, `bounded-contexts.md#note-capture`,
`domain-events.md#note-body-edited`, `validation.md#s3-flush-on-blur`)
are all reflected in either invariants or tests. OK.

### [3] DDD 規約遵守

[3.1] `apps/promptnotes/src-tauri/src/note_capture/slices/flush_note/application.rs:1-108`
pure pipeline, no `std::io`/`std::fs`/`tokio` imports beyond `std::io::Error`
type plumbing. Result-typed errors throughout, no `panic!` /`unwrap` in
the use case. OK.

[3.2] `BodyDiff` is private to the slice (re-declared, not imported from
`auto_save_note`). Matches spec.md#impl-body-diff "slice 間の domain 漏れ
を避けるため、private 再宣言が DDD-VSA 的に正しい". OK.

[3.3] `commands.rs` correctly isolates `tauri::*` to a single file. DTO
boundary types (`FlushTriggerDto`, `FlushErrorDto`, `FlushOutcome`) live
in `commands.rs` only. OK.

### [4] 副作用の境界

[4.1] application.rs steps 1, 7, 8 perform side effects (cancel, write,
publish) via injected ports; steps 2-6 are pure data transforms. Order
matches spec.md#impl-pipeline. OK.

[4.2] `commands.rs:107-114 parse_note_id` swallows parse failures by
falling back to UNIX_EPOCH NoteId, which then triggers `NoteNotFound`.
Spec.md#oq-invalid-note-id explicitly inherits this convention from
auto-save-note. OK as documented, but note this means a malformed
`note_id` from the frontend is observationally indistinguishable from a
nonexistent note in logs/telemetry. (Not a defect; just visible.)

### [5] edge case / テスト網羅

[5.1] **TP-IB tests do not assert cancel was called**.
`apps/promptnotes/src-tauri/src/note_capture/slices/flush_note/tests.rs:548-588`
`tp_ib1` and `tp_ib2` verify `InvalidBody` surfaces and that write/publish
are skipped, but **neither asserts `timer.cancel_count() == 1`**. C-FL1
mandates cancel runs before every other side effect, including the parse
step that surfaces `InvalidBody`. The pipeline does run cancel first in
the source (`application.rs:51`), but the test surface does not pin this
on the invalid-body path. Recommend adding `assert_eq!(timer.cancel_count(), 1)`
to tp_ib2 (or a new assertion in tp_ib1) to symmetrically pin C-FL1 with
how tp_nf2 / tp_co1 do.

[5.2] **TP-LE1 does not assert cancel_count on the load-fail path**.
tests.rs:516-544 asserts write/event are blocked but does not assert
cancel ran. Spec TP-LE2 explicitly says "TP-LE1 のケースで `DebounceTimer::
cancel` は呼ばれる、`NoteRepository::write` も `EventBus::publish` も
呼ばれない". The "cancel は呼ばれる" half is left to tp_co1, but tp_co1
itself (tests.rs:661-673) only asserts `cancel_count == 1` after a
load-failure command — it doesn't verify cancel happened **before** load
(the title says "runs before load" but the body has no ordering check).
There is no FakeRepo `load_order` seq mirroring the trick used for write.
The cancel-before-load invariant is therefore not actually pinned by any
test; it's only inferred from cancel being called *at all* on a load-fail
path.

Recommend either (a) adding a `load_order` channel into `FakeRepo`/`RcRepo`
mirroring how `RcRepo::write` pushes "write" into `timer.seq`, then asserting
`["cancel", "load"]` prefix; or (b) renaming `tp_co1` to clarify it pins
"cancel happens even on load failure" rather than ordering. Currently the
test title (`cancel_runs_before_load`) overclaims.

[5.3] **No coverage for "cancel keyed by the right id on every error path"**.
tp_h2 asserts `last_cancelled == id`. None of NF/LE/IB/PE asserts the
cancelled id matches the command's `note_id`. A bug that cancels the wrong
id on the error paths would slip through. Mild.

[5.4] **No test for "cancel is invoked exactly once per execute call"
on the error paths**. cancel_count is asserted in tp_h2 (=1), tp_h5 (=1),
tp_i2 (=1), tp_nf2 (=1), tp_co1 (=1) but not on LE/IB/PE. If a future
refactor introduced a retry-on-write-failure inside the use case that
re-cancelled, tp_pe* would not catch it. Mild.

[5.5] **No assertion that `FakeRepo.write_order`** (tests.rs:48) is ever
consumed. It's populated in `FakeRepo::write` but only `timer.seq` is
asserted (via `RcRepo`). Dead field. Cleanup, not correctness.

[5.6] **No "Note::edit_body called with clock.now()" assertion on the
S13 trigger** — tp_s13 only asserts cancel→write seq, write_count, and
event_count. The `updated_at` payload check exists in tp_h1/tp_h3 under
BlockBlur only. tp_h5 covers WindowBlur/AppQuit at body level but not
event timestamp. Minor gap; functionally cross-covered.

[5.7] **No idempotency-under-concurrent-trigger test**. If the frontend
dispatches `flush_note(BlockBlur)` and `flush_note(WindowBlur)` for the
same note in rapid succession (e.g., clicking outside a focused block
while the window simultaneously loses focus), both reach the use case.
The second one re-loads, finds the same body the first one persisted,
returns `Ok(None)`. This is *probably* the intended behavior (covered by
tp_i1 conceptually), but the slice does not have an explicit test for
"two-execute-calls-on-the-same-note in sequence is safe and the second
is a no-op". Not strictly required by spec, but spec C-FL11 emphasizes
stateless single-command semantics — pinning two-call composition would
strengthen that claim.

### [6] テスト ↔ spec トレース

[6.1] Every `#[test]` function carries a `/// spec.md#tp-...` doc comment
linking to spec test perspective. Excellent traceability. OK.

[6.2] TP-NF3 ("`id` フィールドは入力の `note_id` をそのまま返す") is
covered as a tail assertion in tp_nf1 (`assert_eq!(id, missing)`). OK.

### [7] 冗長性 / 命名

[7.1] `FakeRepo.write_order` (tests.rs:48) is dead (see [5.5]).

[7.2] `FlushError::PersistError` and `LoadError` both reconstruct the
path via `note_md_path` (`application.rs:91-107`). Identical helper exists
in `auto_save_note/application.rs:79-95`. Acceptable per DDD-VSA slice
independence; flagged for awareness if the slices later need a shared
path-format utility.

[7.3] `commands.rs:107-114 parse_note_id` is a verbatim duplicate of the
auto-save-note boundary handler (per the inline `oq-invalid-note-id`
note). Worth extracting only if a third slice needs it; current duplication
is intentional.

### [8] 上流ドメインとの差分（先取り spec の安全性）

[8.1] spec.md#io-errors deliberately preempts the 4-variant alignment
that `domain/workflows/flush-note.md#errors` does not yet declare (only
`NoteNotFound`, `PersistError`). The spec records this as
`oq-error-variant-alignment` for phase-7 propose. Production code
implements the 4-variant. This is the same upstream-first dance the
auto-save-note slice resolved; the proposal is queued.

Important: **the slice will not be coherent until that proposal lands
upstream**. If phase 7 finalize closes status.yaml dirty without filing
the propose, the next `/ori-sync` will flag a hash drift. Reviewer
acknowledges spec records this; flagging for phase-7 follow-through.

[8.2] status.yaml shows `current_phase: null` and `completion: []` even
though 115 tests are GREEN. Phase tracking is not being updated as the
flow advances. Cosmetic for review, but `/ori-finalize` must rebuild
this.

## 総合判定

**NEEDS_FIX**

理由:

1. **[1.1]** C-FL1 の production semantics が NoOpDebounceTimer で空洞化
   している点を spec で開示する（OQ 追加 or invariant の文言緩和）。
   現状の spec は「cancel → load → ... → persist の順序固定で AutoSave
   並走防止」と断言しているが、production composition root は cancel が
   no-op で、契約は JS 層に丸投げされている。これは domain invariant の
   宣言と実装の事実が乖離している状態であり、reviewer 観点で最も重い
   指摘。
2. **[5.1] [5.2]** C-FL1 のテスト pin が脆い。InvalidBody 経路で cancel
   呼び出しを assert しておらず、tp_co1 は "before load" を pin して
   いないため、cancel-first ordering の test contract が部分的に空。
   タイトル `cancel_runs_before_load` は overclaim。
3. **[2.1]** S13 の Rust 側 quit orchestration が未実装で、`lib.rs` に
   `on_window_event` フックがない。spec out-of-scope に明記されている
   ものの、derives_from が `s13-quit-flush` を含む以上、cross-slice
   integration として beads issue を切るのが筋。

軽微（修正なくても PASS は許容、ただし phase 7 で拾うのが望ましい）:
- [5.3] 各 error 経路で cancel された id の一致 assert
- [5.4] cancel_count == 1 を error 経路でも pin
- [5.5] dead `FakeRepo.write_order` の削除
- [5.7] 2 連続 execute（trigger 競合）の冪等性 test
- [8.2] status.yaml の phase tracking

PASS にできない核心は [1.1]。spec 文言と production wiring の事実関係を
そろえる（OQ 追加または invariant の緩和）まで、本 slice は domain
contract と impl の整合性を満たさない。

---

## Pass 2 (orchestrator patch summary)

Pass 1 NEEDS_FIX を受けた patch:

### 1. spec.md amendments
- **C-FL1 wording 緩和**: 「AutoSave 並走防止」を invariant から外し、composition root (UI 層 timer) の責務として再帰属。Rust use case 側の C-FL1 は「`DebounceTimer` port を最初に同期呼び出しする」契約に限定。
- **新規 OQ `#oq-debounce-cancel-composition`**: production の NoOpDebounceTimer 採択根拠と、frontend hook 側に責務を委譲する trade-off を明文化。万一 cancel skip 時の disk I/O 2 回シナリオも記載。

### 2. テスト pin の強化
- **`tp_co1_cancel_runs_before_load`**: `timer.seq` に `RcRepo::load_by_id` の `"load"` を mirror し、`["cancel", "load"]` 順序を直接 assert。
- **`tp_h2`, `tp_s13_app_quit_flush_single_note_succeeds`**: 既存 `["cancel", "write"]` を `["cancel", "load", "write"]` に更新（load step も seq に乗るため）。
- **`tp_nf2`**: `["cancel", "load"]` seq と `last_cancelled == id` を追加 pin。
- **`tp_le1`**: `timer.cancel_count() == 1` と `last_cancelled == id` を追加 pin。
- **`tp_ib2`**: `timer.cancel_count() == 1` と `last_cancelled == id` を追加 pin（C-FL1 が parse step より先に走ることを pin）。
- **`tp_pe3`**: `timer.cancel_count() == 1` と `last_cancelled == id` を追加 pin。

### 3. dead field 削除
- `FakeRepo::write_order` を削除。`RcRepo` が `timer.seq` へ mirror するため不要。

### 4. cross-slice integration の追跡
- bd `ori-73q` を新規発行: 「Note Capture: Tauri 側 quit orchestration を実装 (S13 連続 Flush)」。`lib.rs` の `on_window_event(WindowEvent::CloseRequested)` フック実装は cross-slice integration として別 issue で追跡。

### 5. 残置事項

軽微指摘の処理:
- [5.5] dead field 削除済
- [5.3] [5.4] エラー経路で `cancel_count == 1` と `last_cancelled == id` を追加 pin 済
- [5.6] (S13 trigger 下での event timestamp 直接 assert) — tp_h3 + tp_h5 で間接 cover 済、明示 test は今回追加せず
- [5.7] (二連続 execute 冪等性) — 概念的に tp_i1 / tp_pe4 で cover 済
- [8.2] (status.yaml current_phase: null) — phase 7 finalize で更新

これらは PASS 阻害しない。

### 6. Pass 2 を再 reviewer に委ねる

`/ori-flow` skill 規約に従い orchestrator は patch 後の独立 reviewer 起動を実施 (max 1 round)。

---

## Pass 2 reviewer verdict

Pass 1 NEEDS_FIX を発行した同一 reviewer による独立判定。115/115 tests GREEN, cargo fmt clean を前提に、orchestrator patch が Pass 1 blocker を実際に閉じたかを検証した。

### Pass 1 blocker の解消確認

**[1.1] C-FL1 production race-prevention の整合性** — **CLOSED**

`.ori/slices/flush-note/spec.md#invariants-slice-specific` の C-FL1 文言が、port-level な「`DebounceTimer` port を最初に同期呼び出しする」契約に再定式化された。旧文言「AutoSave 並走防止」は composition root（UI 層 debounce timer）責務として再帰属され、`#oq-debounce-cancel-composition` (`.ori/slices/flush-note/spec.md:368-378`) に NoOpDebounceTimer 採択根拠・trade-off・skip 時の disk I/O 2 回シナリオまで明記された。Pass 1 の recommendation (a) が直接採用された形であり、domain invariant と production wiring の不整合は解消。OQ は status: open のまま phase-7 で正規化されるが、これは review 観点では PASS 阻害しない（OQ で明示的に未決と宣言された境界条件は invariant 違反ではない）。

**[5.1] tp_ib2 が cancel を pin しない** — **CLOSED**

`apps/promptnotes/src-tauri/src/note_capture/slices/flush_note/tests.rs:598-605` で `tp_ib2` が `timer.cancel_count() == 1` および `timer.last_cancelled() == id` を assert している。C-FL1 が parse step より先に走ることが test surface で pin されている。

**[5.2] tp_co1 が cancel-before-load を pin しない** — **CLOSED**

`tests.rs:162-168` で `RcRepo::load_by_id` が `timer.seq` に `"load"` を mirror するようになり、`tp_co1_cancel_runs_before_load` (`tests.rs:689-705`) が `timer.seq == ["cancel", "load"]` を直接 assert している。test 名が示す順序契約 (cancel **before** load) が実装上の seq で観測可能になった。さらに `tp_h2` / `tp_s13_app_quit_flush_single_note_succeeds` / `tp_nf2` が `["cancel", "load", "write"]` / `["cancel", "load"]` を assert することで、ordering invariant が全主要経路で symmetric に pin されている。test title の overclaim は解消。

**[2.1] S13 quit orchestration の follow-up issue 未起票** — **CLOSED**

beads `ori-73q` が起票済み (`bd show ori-73q` で確認: 「Note Capture: Tauri 側 quit orchestration を実装 (S13 連続 Flush)」、`on_window_event(WindowEvent::CloseRequested)` 含む)。derives_from に `s13-quit-flush` が残る限り cross-slice integration として tracking 義務を持つが、その義務は beads issue で果たされた。

### 軽微指摘の処置確認

- **[5.3]** error 経路で `last_cancelled == id` 追加 → tp_nf2 / tp_le1 / tp_ib2 / tp_pe3 で pin 済。**CLOSED**
- **[5.4]** error 経路で `cancel_count == 1` 追加 → 上記 4 test で pin 済。**CLOSED**
- **[5.5]** dead `FakeRepo::write_order` → 削除済 (`tests.rs` Pass 2 版に該当 field なし)。**CLOSED**
- **[5.6]** S13 trigger 下の event timestamp 直接 assert → tp_h3 + tp_h5 で間接 cover、明示 test なし。**KNOWINGLY DEFERRED** (orchestrator 判断、PASS 阻害なし)
- **[5.7]** 二連続 execute 冪等性 test → tp_i1 / tp_pe4 で概念 cover。**KNOWINGLY DEFERRED** (同上)
- **[8.2]** status.yaml current_phase: null → phase-7 finalize で更新予定。**DEFERRED** (phase 区分上正当)

### Regression / 新規 drift 検査

1. **OQ 化が C-FL1 を骨抜きにしたか**: しなかった。C-FL1 は依然として「port を最初に同期呼び出しする」hard invariant として残り、`tp_co1` / `tp_h2` / `tp_nf2` / `tp_le1` / `tp_ib2` / `tp_pe3` で pin されている。「何を cancel するか」と「いつ port を呼ぶか」を分離した型理論的に健全な改定。
2. **OQ wording の自己矛盾**: なし。`#oq-debounce-cancel-composition` は trade-off 3 案（NoOp / Rust notify / auto_save_note guard）を列挙し、現状の NoOp 採択根拠と影響範囲（冪等性ガードで event は重複しないが disk I/O 2 回）を明示。Pass 1 指摘の「契約の事実関係をそろえる」要件を満たす。
3. **test pin の cosmetic vs genuine**: genuine。`timer.seq` への `"load"` mirror は `RcRepo::load_by_id` が呼ばれた事実を観測可能にし、cancel→load の strict precedence を「cancel が呼ばれた／load が呼ばれた」の単純な count assertion から「順序付き trace」へ格上げしている。これは Pass 1 で指摘した「cancel-before-load の test contract が空」の直接的な是正である。
4. **production code unchanged の妥当性**: 妥当。Pass 1 の指摘はすべて (a) spec wording, (b) test pin, (c) cross-slice issue triage の 3 軸で、impl 変更を要求していない。production の 4-variant `FlushError` / pipeline / Tauri surface / `DebounceTimer` port は Pass 1 時点で OK と判定されていた。

### 総合判定

**PASS**

理由:

1. Pass 1 の重い blocker ([1.1], [5.1], [5.2], [2.1]) はすべて closed。spec 文言と production wiring の事実関係が `#oq-debounce-cancel-composition` 経由で明示的に一致し、test pin は cancel-before-load を全主要経路で trace asssert で固定している。
2. 軽微指摘の deferral (`[5.6]`, `[5.7]`, `[8.2]`) は orchestrator が明示判断しており、review 観点で PASS 阻害しない範囲。
3. 新規 regression / drift は検出されず、115/115 tests GREEN, cargo fmt clean。
4. cross-slice integration (`ori-73q`) が beads で tracking 開始されたことで、derives_from `s13-quit-flush` の satisfaction 経路が宙に浮かない。
5. phase-7 propose の対象（`#oq-error-variant-alignment` および `#oq-debounce-cancel-composition`）は spec 上で明示宣言されており、`/ori-finalize` が拾うべき contract が clear。

本 slice は merge / finalize 可能。
