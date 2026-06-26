# restore-deleted-note review

## Pass 1

Adversarial review on a fresh context. Inputs: `.ori/slices/restore-deleted-note/{spec.md,manifest.yaml}`, implementation under `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/`, BC-shared changes (`shared/events.rs`, `delete_note/ports.rs`, `delete_note/tests.rs`), `slices/mod.rs`, `lib.rs`, and the relevant domain docs (workflows/restore-deleted-note.md, aggregates.md#notes-undo, domain-events.md#note-restored-from-trash, validation.md S5/S6/S7). 218 lib tests + 13 slice tests GREEN as reported.

Findings are grouped per the 7 review axes. Severity legend: HIGH = spec/domain violation or correctness gap that should block merge; MED = spec drift or hidden regression risk that should be addressed; LOW = polish / future-proofing. Disposition at the bottom.

### [1] spec ↔ impl consistency

[1] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/application.rs:94` — **MED** `note_id_for_event(restored.id())` clones `restored.id()` for the event, but the input `note_id` was moved (`let RestoreDeletedNoteCommand { note_id } = cmd;`) and is still alive. The function adds zero value: it's a 1-liner clone wrapped in a free function, and it hides the fact that the event's `note_id` is taken from the **reloaded** note, not from the input command. spec I-RDN9 and TP-EP1 both implicitly require the event payload's `note_id` to equal the input. If `NoteRepository::load_by_id` ever returned a `Note` whose `id` differed from the requested `&NoteId` (e.g. a future repo refactor), this code would silently emit an event for the wrong id without any test catching it (TP-EP1 only asserts `note_id == id` where both come from the same `original`). Recommend: drop `note_id_for_event` and publish with `note_id: note_id.clone()` (input-derived) — that is what spec I-RDN9 / domain-events.md#note-restored-from-trash-payload implies and what S5/S7 reason about.

[2] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/application.rs:81-87` — **LOW** the Ok(None) → ReadError collapse uses `io::Error::new(NotFound, "restored .md not found by load_by_id (post-restore inconsistency)")`. spec I-RDN4 / oq-read-error-ok-none-policy say the policy is "事後不整合の表面化". The chosen `ErrorKind::NotFound` is reasonable, but the spec wording would also support a more specific sentinel kind (e.g. `Other` with the explanatory message). Not a blocker — the test `tp_re2` pins NotFound, so spec and impl match. Just be aware that a future propagation to UI may want to distinguish "pre-existing missing" from "post-restore disappearance".

### [2] derives_from coverage

manifest declares 6 upstream sections and the spec frontmatter mirrors them. Verified coverage:

- `domain/workflows/restore-deleted-note.md#restore-deleted-note` — 5 Steps (find / restore / reload / remove / emit) reflected as I-RDN1..I-RDN6 and the 5-step pipeline in `application.rs`. **OK**
- `domain/aggregates.md#note-aggregate` — I-N1..I-N7 non-violation argued in spec; impl never mutates aggregate fields (read-only via `from_persisted` inside the repo). **OK**
- `domain/bounded-contexts.md#note-capture` — write-side Undo path lives in Note Capture BC; slice placement under `note_capture/slices/restore_deleted_note/` is correct. **OK**
- `domain/domain-events.md#note-restored-from-trash` — `DomainEvent::NoteRestoredFromTrash { note_id, restored_at }` variant added with matching payload. **OK**
- `domain/validation.md#s5-delete-undo-in-window` — pinned by `tp_h1` (happy path) + `tp_so1` (5 side-effect order) + `tp_ep1` (event payload at `Clock::now`). **OK**
- `domain/validation.md#s7-undo-after-toast` — pinned by `tp_nu1` (empty stack) + `tp_nu2` (different id retained) + `tp_nn1` (read-only path). **OK**

[3] `.ori/slices/restore-deleted-note/spec.md:17-18` — **LOW** The two validation hashes are identical (`5294b0c32f1b`) for `s5-delete-undo-in-window` and `s7-undo-after-toast`. That's almost certainly a single file-level hash for `validation.md` rather than per-section. Not introduced by this slice (same pattern likely used by delete-note as well), but worth flagging because if S5 changes without S7 changing, the slice will mark both dirty / clean together. Recommend: investigate during `/ori-sync` for whether section-level hashing is supported.

### [3] DDD-VSA-Hex conformance

- `domain.rs` only holds the command + error enum, no I/O. **OK**
- `application.rs` is generic over 5 ports, no concrete I/O type leaks in. **OK**
- `commands.rs` is the only place that imports `tauri`. **OK**
- pure path uses `Result<Note, RestoreDeletedNoteError>`, no `panic!` / `expect` in the application pipeline. **OK**

[4] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/commands.rs:142-146` — **MED** `parse_note_id` falls back to `NoteId::from_timestamp(UNIX_EPOCH)` when the raw string fails to parse. This silently turns a malformed input into a deterministic non-existent id and will be reported back as `NoUndoAvailable { id: "19700101000000" }`. That is wrong for two reasons:
  1. It maps a **client error** (bad input) to a **domain outcome** (`NoUndoAvailable`), violating layer separation. Domain Modeling Made Functional explicitly puts input parsing at the boundary and returns parse errors *before* invoking the workflow.
  2. The UI then can't distinguish "user passed garbage" from "the Undo window expired".
  Recommend: return `Err(RestoreDeletedNoteErrorDto)` with a fourth variant (e.g. `InvalidNoteId { raw }`) at the Tauri boundary, or — since the production wiring is deferred anyway — make `parse_note_id` return `Result` and propagate it as the existing `NoUndoAvailable` with an `InvalidNoteId` discriminator. This same pattern is also worth checking on delete-note's command surface (out of scope here but the parent should triage).

### [4] side-effect placement

- `find_by_id` is a query but is the only step that can `?`-exit without further effects; `remove_by_id` is the corresponding mutation, kept after reload as spec demands. **OK**
- Event publish is the last step inside `execute`, gated by 4 prior successes (I-RDN6). **OK**
- No `println!` / `eprintln!` / logging mixed into pure pipeline. **OK**

### [5] edge case coverage in tests

[5] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/tests.rs:421-451` (`tp_tr1`) and `tests.rs:485-516` (`tp_re2`) — **MED** When `restore_from_trash` succeeds but reload fails (`tp_re1` / `tp_re2`), the OS-trash side effect has already happened — the file is back on disk — yet the spec keeps the `DeletedNote` on the Undo stack (I-RDN4) so the user can retry. The tests verify `undo.snapshot().len() == 1`, but they do *not* assert that the Undo stack entry still points at the **same `original_path`** (no replacement, no path corruption). Lightweight, but adds robustness:
```rust
assert_eq!(undo.snapshot()[0].id(), &id);
assert_eq!(undo.snapshot()[0].original_path(), path.as_path());
```
The same retry-safety contract on `tp_tr1` is missing too — only stack length is asserted.

[6] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/tests.rs:189-198` — **MED** `FakeUndo::find_by_id` only pushes `"find"` to the order_log **when the id is found**. That is fine for the happy-path `tp_so1` assertion, but it means the no-undo path (`tp_nu1` / `tp_nu2` / `tp_nn1`) has no positive evidence that `find_by_id` was called *at all*. If a future refactor accidentally bypassed `find_by_id` and went straight to `trash`, the existing `find_count` cell would catch it (since it increments unconditionally), but the order-log spy contract becomes asymmetric. Recommend: always push `"find_miss"` (or always push `"find"` and then have the order assertion ignore misses), so the spy log behaves uniformly. Not blocking — `find_count` already provides the same evidence — but the asymmetry is a smell.

[7] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/tests.rs:83-91` — **MED** `FakeRepo::load_by_id` increments `load_count` unconditionally but pushes `"load"` to the order_log only on the `Ok` path. Same asymmetry as above. For `tp_re1` (io error) the order_log will show `["find", "trash"]` — no entry for the failed load — so the test does not pin that the load happened **between** trash and remove on the failure path. Combined with finding [6], the spy substrate has two related blind spots. Recommend: always push to order_log even on failure (`"load_err"` is fine) so the order contract on failure paths can also be asserted.

[8] missing TP: **MED** there is no test for the scenario "another, **different** `DeletedNote` is present on the stack and reload of the target fails" — i.e. failure path must not affect siblings on the stack (the symmetric companion to `tp_tp1`). Sketch:
```
seed [A, B]; fail reload for B → undo.snapshot() == [A, B] in order; no event
```
This pins I-RDN4 + I-RDN7 together. Currently I-RDN7 is only proven on the happy path.

[9] missing TP: **LOW** `oq-duplicate-deleted-note-by-id` is explicitly flagged as undefined behavior in spec I-RDN9. A pinning test (even a `#[test] #[ignore]` documenting the current "first-match" semantics) would prevent silent regressions when `delete-note` evolves. Not required since spec marks it OQ, but useful for the next iteration.

[10] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/tests.rs:296-299` — **LOW** `make_deleted` uses `note.delete_to_trash(path)` which is the **production** aggregate command. That is good (matches the delete-note `H-1` review fix) — calls out only to note that the same convention is preserved here. **OK**

### [6] test ↔ spec traceability

Every `#[test]` carries a comment with `tp-...` and / or invariant id. Cross-check:

| test | spec TP | invariants |
|---|---|---|
| `tp_h1_*` | tp-happy | I-RDN5/6/7/8 (partial) |
| `tp_nu1_*` | tp-no-undo-empty + tp-no-undo-noop | I-RDN1 |
| `tp_nu2_*` | tp-no-undo-different-id | I-RDN1 + I-RDN7 |
| `tp_tp1_*` | tp-stack-targeted-pop | I-RDN7 |
| `tp_tr1_*` | tp-trash-restore-err | I-RDN3 |
| `tp_re1_*` | tp-read-err-io | I-RDN4 |
| `tp_re2_*` | tp-read-err-ok-none | I-RDN4 + oq-read-error-ok-none-policy |
| `tp_so1_*` | tp-side-effect-order | I-RDN5 |
| `tp_ep1_*` | tp-event-payload | I-RDN8 |
| `tp_rs1_*` | tp-restored-note-shape | (workflow#notes) |
| `tp_pd1_*` | tp-path-from-deleted-note | I-RDN2 |
| `tp_nn1_*` | tp-no-undo-noop | I-RDN1 |
| `tp_sig_*` | (signature pin) | — |

Coverage is **strong**. The only TP not directly pinned by name is `tp-read-err-ok-none-policy`'s "delete-note との差別化" claim — but the difference is visible in code review since both slices live in the same crate. **OK**.

### [7] redundancy / premature abstraction

[11] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/application.rs:102-104` — **LOW** `fn note_id_for_event(id: &NoteId) -> NoteId { id.clone() }` is a free function that wraps a single `.clone()`. Either inline it (`note_id: restored.id().clone()`) or — better, per finding [1] — replace its callsite with the input-derived `note_id.clone()`. The current free function adds a misleading abstraction layer.

[12] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/commands.rs:24-27` — **LOW** `NoOpBus` silently drops events at the Tauri boundary. The spec contract says the event is published on success (I-RDN6). With `NoOpBus`, the production wiring is functionally event-less. Same situation as delete-note (deferred per its follow-up), so this is consistent — but please file a follow-up beads issue (or confirm the existing delete-note follow-up covers both) so the prod EventBus wiring is tracked. Acceptable as deferred, but should not be forgotten.

[13] `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/ports.rs:38-44` — **LOW** TrashService extension: putting `restore_from_trash` and `move_to_trash` on the same trait (per oq-trash-service-extension) means every adapter must implement both. For the production OS adapter that's fine (`trash` crate or NSWorkspace exposes both). For test doubles in delete-note that don't care about restore, this forces an `unreachable!()` arm. The cleaner ISP-compliant alternative (split into `TrashMove` + `TrashRestore`) is mentioned in the OQ; current decision is justified in spec. **Accepted** as documented choice, but flagging that the cost has materialized in `delete_note/tests.rs` (FakeTrash / FakeUndo `unreachable!()`).

[14] `apps/promptnotes/src-tauri/src/note_capture/slices/delete_note/ports.rs:38-44` — **MED** `UndoStack::find_by_id` returns `Option<DeletedNote>` and the slice immediately converts `None` to `NoUndoAvailable { id }`. The OQ asks whether `Result<DeletedNote, NoUndoAvailable>` would be more intent-explicit. Current `Option` is fine for a port (the port doesn't need to know the slice's error type), but the API shape **forces** `find_by_id` to **clone** the `DeletedNote` out, even though `remove_by_id` will re-traverse and pop the same element seconds later. Two traversals + a clone for what could be a single "take" operation. Recommend either:
  - keep current shape but document that the clone is intentional (audit trail in spec I-RDN9), or
  - replace `find_by_id` + `remove_by_id` with a single `take_by_id(&NoteId) -> Option<DeletedNote>` that pops in one shot, and let the slice perform `restore_from_trash` + `load_by_id` on the popped value, with rollback (re-push) on failure. This is closer to the typical Undo-as-pop pattern.
  The current 2-step approach is **correct** per spec — and rollback-on-failure is harder to reason about — so this is design feedback rather than a blocker. spec I-RDN4 explicitly chose "leave on stack" semantics on reload failure, which fits today's API. Document the trade-off in the OQ when next revising.

### Production wiring sanity

- `apps/promptnotes/src-tauri/src/lib.rs:29` registers `restore_deleted_note` in `invoke_handler`. **OK**
- `apps/promptnotes/src-tauri/src/note_capture/slices/mod.rs:8` declares `pub mod restore_deleted_note;`. **OK**
- `RestoreDeletedNoteErrorDto` covers all 3 `RestoreDeletedNoteError` variants. **OK** (would need a 4th if finding [4] is addressed).

### Disposition

**NEEDS_FIX**

Reasoning (severity-ordered):

1. **[4] MED** `commands.rs::parse_note_id` collapses invalid input to a domain `NoUndoAvailable`, masking a client-side parse error as a domain outcome. Layer-of-concern violation. Should at least emit a distinct DTO variant.
2. **[1] MED** `application.rs` derives the event's `note_id` from the reloaded Note rather than from the input command. spec I-RDN9 + S5/S7 reason about the input id. Trivial fix; matters for invariant integrity.
3. **[6] [7] MED** order_log spy is asymmetric (`find_miss` and `load_err` not recorded), leaving failure-path side-effect order partially unverifiable.
4. **[5] MED** retry-safety on failure paths is asserted only by stack length; add `original_path` equality to `tp_tr1` / `tp_re1`.
5. **[8] MED** missing TP for "failure path leaves siblings on stack intact" (I-RDN4 ∧ I-RDN7 combined).
6. Findings [2], [3], [9]–[14] are LOW — polish, future-proofing, or accepted-trade-off documentation. Address opportunistically.

None of the MED findings demonstrate that the current GREEN tests are wrong — they show that the test substrate has blind spots and the Tauri boundary has an inputs-as-domain-state smell. Fix #1 and #2 are 1-line changes. #3–#5 are localized test additions. After those, the slice should be PASS.

This is Pass 1; per `/ori-flow` protocol the reviewer does not re-review. Apply the fixes (or explicitly defer with beads issues + spec OQ entries) and proceed to phase 7 finalize.

## Pass 2

Fresh-context adversarial pass against Pass 1 NEEDS_FIX. Verified inputs: `application.rs`, `commands.rs`, `tests.rs` (post-patch), `spec.md`, `manifest.yaml`. 14/14 slice tests + 219/219 lib tests GREEN locally re-confirmed.

### Pass 1 MED resolution verification

| Finding | Fix location | Status |
|---|---|---|
| MED-1 (event note_id from reloaded Note) | `application.rs:53,96-99` — `note_id` destructured from input, `bus.publish(... note_id: note_id.clone() ...)`; helper `note_id_for_event` removed | **RESOLVED**. Inline comment at L93-95 cites Pass 1 MED-1 and the design rationale (input is authoritative source). Existing `tp_ep1` continues to assert `note_id == id` where `id` is also input-derived, so the test remains valid even though it doesn't yet *exploit* the new contract (i.e. a hypothetical `load_by_id` returning a different-id Note would still pass `tp_ep1` because both sides use input id — but the impl change is correct and matches I-RDN9 reading). |
| MED-4 (commands.rs parse fallback to UNIX_EPOCH) | `commands.rs:69-89` adds `InvalidNoteId { raw: String }` DTO variant; `commands.rs:122-128` returns it on parse Err; old `parse_note_id` helper removed | **RESOLVED**. Layer separation restored: client parse error now distinct from `NoUndoAvailable` domain outcome. DTO `From` impl at L91-107 covers the 3 domain variants; `InvalidNoteId` is intentionally not produced from `RestoreDeletedNoteError` because it lives only at the Tauri boundary — that is correct. |
| MED-5 (retry-safety same DeletedNote on failure paths) | `tests.rs:460-465` (tp_tr1) + `tests.rs:499-501` (tp_re1) add `snapshot[0].id() == &id` and `snapshot[0].original_path() == path` | **RESOLVED**. Both failure paths now pin id+path equality on the retained DeletedNote, preventing silent corruption of the retry payload. |
| MED-6 (FakeUndo::find_by_id asymmetric log) | `tests.rs:189-199` — `order_log.push("find")` moved outside the `find().cloned()` chain so it fires for both hit and miss; `tp_nu1` at L355-360 adds `assert_eq!(observed, vec!["find"])` | **RESOLVED**. Spy substrate is now symmetric for `find_by_id`. tp_nu1 provides positive evidence that find_by_id is the gating step on the empty-stack path. |
| MED-7 (FakeRepo::load_by_id asymmetric log) | `tests.rs:83-92` — `order_log.push("load")` runs before `fail_load_with` branch | **RESOLVED**. `load` now appears in the log on both Ok and Err paths. (Note: no test currently asserts the failure-path order including "load"; the substrate is ready for it but tp_re1/tp_re2 still rely on counters. Not a regression — opportunity LOW-A below.) |
| MED-8 (sibling intact on failure path) | `tests.rs:505-533` — new `tp_tr2_trash_failure_keeps_sibling_deleted_intact`. Two-element seed, restore B fails via `Io("disk full")`, asserts both elements remain at correct indices with id+path equality, plus `bus.event_count() == 0` | **RESOLVED**. Combined I-RDN4 ∧ I-RDN7 on a failure path is now structurally pinned. Companion to existing `tp_tp1` (happy-path targeted pop). |

All five Pass 1 MED findings resolve cleanly. The patches are minimal and targeted — no over-correction.

### Regression scan (Pass 1 → Pass 2 patches)

[15] **`tp_so1` order assertion stability** — With `find_by_id` and `load_by_id` now logging unconditionally, the happy-path observed log on `tp_so1` could in principle change. Verified at `tests.rs:585-590`: the assertion `vec!["find", "trash", "load", "remove", "event"]` matches because the happy path produces exactly one `find` (success), one `trash` (success — `FakeTrash::restore_from_trash` only logs on Ok, L140), one `load` (success), one `remove` (Some path, L205), one `event`. Re-confirmed by GREEN run. **No regression.**

[16] **`FakeTrash::restore_from_trash` retains the asymmetry that Pass 1 flagged for find/load** — `tests.rs:136-143`: the failure path early-returns at L138 *before* `order_log.push("trash")` at L140. The Pass 1 patch list did not include this (only undo + repo), but the spy contract is now inconsistent across the three substrates: undo and repo log unconditionally, trash logs only on success. No current test depends on a failure-path `"trash"` log entry, so this is **LOW**, not a regression. Recommend bringing FakeTrash in line on the next opportunity (would need a `trash_err` token to distinguish or always push `"trash"` regardless of outcome).

[17] **`tp_nu1` log assertion is exact-match** — `tests.rs:355-360` uses `assert_eq!(observed, vec!["find"])`. This pins not only that find ran, but that *nothing else* ran. Good — this is strictly tighter than Pass 1's `find_count == 1`, and aligns with I-RDN1. **Improvement**, no regression.

### Pass 1 LOW residuals

| LOW | Status in Pass 2 | Notes |
|---|---|---|
| [2] `ErrorKind::NotFound` for Ok(None) collapse | Unchanged. Acceptable per `oq-read-error-ok-none-policy`. | No action required this slice. |
| [3] validation.md s5/s7 share the same hash `5294b0c32f1b` | Unchanged. Spec frontmatter still shows identical hashes at L17-18. | Cross-slice concern; flag in `/ori-sync`. |
| [9] missing pinning test for `oq-duplicate-deleted-note-by-id` | Unchanged. Spec marks as OQ; deferral acceptable. | — |
| [10] `make_deleted` uses production aggregate command | Unchanged, **OK**. | — |
| [11] `note_id_for_event` free function | **RESOLVED** as collateral of MED-1 fix (function deleted). | — |
| [12] `NoOpBus` swallows events in prod wiring | Unchanged. Same status as delete-note; should be tracked as follow-up beads. | Confirm coverage. |
| [13] TrashService trait inflation forces `unreachable!()` in test doubles | Unchanged. Documented OQ. | — |
| [14] find+remove vs single take_by_id | Unchanged. Documented design choice. | — |

One LOW (#11) resolved as a side effect of MED-1. The rest remain as previously dispositioned (acceptable / deferred).

### New Pass 2 findings

[LOW-A] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/tests.rs:83-92` — **LOW** Now that `load_by_id` logs unconditionally, a natural next step is to upgrade `tp_re1` to assert the failure-path order, e.g. `assert_eq!(observed, vec!["find", "trash", "load"])`. Not necessary for current GREEN guarantees (counters already pin remove_count/event_count == 0), but would close the symmetry that MED-7 set up. Opportunistic.

[LOW-B] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/tests.rs:136-143` — **LOW** `FakeTrash::restore_from_trash` failure path bypasses the order_log. This is the same shape as Pass 1 MED-6/MED-7 but for the trash spy. Since no test currently asserts the failure-path order including `"trash"`, this does not cause a failing scenario today — but the spy substrate's contract is now non-uniform (undo / repo log always, trash logs only on success). If LOW-A is taken, this should be tightened too so a `tp_tr1`-style failure order assertion (e.g. `["find"]` — trash entry would be omitted on failure) becomes meaningful. Recommend: align FakeTrash with the new always-log convention (e.g. push `"trash"` before the early-return, or push `"trash_err"` on the failure branch).

[LOW-C] `apps/promptnotes/src-tauri/src/note_capture/slices/restore_deleted_note/commands.rs:38-41` — **LOW** `UnimplementedTrash::restore_from_trash` returns `TrashErrorKind::Unsupported`. Combined with `UnimplementedUndo::find_by_id` returning `None`, the production wiring will currently always fail at the find_by_id step with `NoUndoAvailable` — `restore_from_trash` will never be reached. That is consistent with the deferred-prod-wiring stance shared with delete-note, but worth a beads follow-up so the placeholder is replaced before this command is wired to the UI. (Existing Pass 1 [12] covers `NoOpBus`; this is the same family of deferrals.)

None of the new Pass 2 findings rise to MED. No HIGH found.

### Disposition

**PASS**

Reasoning:

1. All five Pass 1 MED findings (1, 4, 5, 6, 7, 8) are resolved with targeted, minimal patches that match the stated intent.
2. No regression detected from the patches — `tp_so1` order assertion remains valid under the new always-log spy contract; all 14 slice tests + 219 lib tests GREEN.
3. Pass 1 LOW [11] resolved as collateral. Remaining LOWs are documented or deferred trade-offs.
4. New Pass 2 findings are all LOW and concern opportunistic test-substrate symmetry (LOW-A, LOW-B) or pre-existing deferred prod wiring (LOW-C). None block merge.

Slice is ready for phase 7 finalize. The two new LOW items (LOW-A, LOW-B) can be addressed opportunistically in a future slice or carried as a small follow-up beads issue; LOW-C should be tracked alongside the delete-note prod-wiring follow-up.
