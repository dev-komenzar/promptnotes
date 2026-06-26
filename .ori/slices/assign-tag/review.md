# Review: assign-tag {#review-assign-tag}

## Pass 1 {#pass-1}

Reviewer: Claude Opus 4.7 (1M), fresh context
Scope: spec.md + manifest.yaml + 6 upstream domain sections + impl under
`apps/promptnotes/src-tauri/src/note_capture/slices/assign_tag/` + shared
deltas (`events.rs`, `types/note.rs`) + sibling `auto-save-note/review.md`
for comparative reference.

Test run: `cargo test --lib` → **112 passed; 0 failed**. Cross-slice GREEN
confirmed empirically (auto-save-note / copy-note-body / create-note all
still pass after `events.rs` variant addition and `Note::assign_tag` aggregate
extension).

### Findings {#findings}

#### Open question follow-through (MED, but explicitly promised)

- **MED-1** `oq-assign-tag-now-injection` / `oq-tag-new-signature`: spec.md
  lines 300, 309 promise "**phase 6 で上流 proposal を作成する**" for both
  OQs. No proposals exist under `.ori/proposals/pending/` for assign-tag (only
  the accepted auto-save-note ones). The implementer extended
  `Note::assign_tag(self, tag, now)` (note.rs:57) — a real divergence from
  `aggregates.md#note-aggregate-commands` line 78 which still declares
  `Note::assign_tag(self, tag: Tag) -> Note`. Similarly `TagError::Empty`
  vs domain's `EmptyAfterTrim` (tag.rs:13 vs validation/workflow text) is
  unreconciled.
  - Detail: This is the same pattern auto-save-note used (cross_slice_followups
    → proposals), but assign-tag finalize hasn't run yet — phase 7 should
    surface these. Verify ori-finalize lifts them; otherwise the divergence
    becomes silent debt.
  - Recommendation: **bounce to phase 7 (finalize)** with explicit reminder
    to file 2 proposals (`aggregates-note-aggregate-assign-tag-signature`
    and `workflows-assign-tag-tag-error-naming`). Not a blocker for the impl
    itself.

#### Aggregate behavior divergence on no-op path (MED)

- **MED-2** `Note::assign_tag` no-op branch drops `updated_at` update:
  note.rs:58–61 returns `self` unchanged when the tag already exists.
  `aggregates.md` line 78–80 explicitly states "既存なら no-op、I-N5。
  `updatedAt` は **更新する**（tags も frontmatter 経由で永続化されるため）".
  - Detail: In this slice the application service short-circuits before
    calling the aggregate's no-op branch, so the slice itself behaves
    correctly (TP-N2 / TP-N3 GREEN). But the aggregate is **shared**
    (Note Capture Shared Kernel). A future caller that invokes
    `Note::assign_tag` without the service-level diff will silently violate
    the documented "updatedAt は更新する" contract.
  - Recommendation: Either (a) update the aggregate to set
    `updated_at = now` even on no-op (matches domain doc, breaks no current
    test), or (b) file a proposal to amend `aggregates.md` to reflect the
    new contract "no-op skips updatedAt; application service owns dedupe".
    Option (b) aligns with the existing design choice. **Bounce to phase 7**
    along with MED-1.

#### CJK / multibyte tag coverage gap (MED)

- **MED-3** test coverage skips the CJK happy-path and multibyte edge cases:
  `aggregates.md#note-aggregate-elements` says Tag は CJK 許容, and
  `glossary.md` allows non-ASCII tag names, but **no test asserts that
  `Tag::new("日本語")` or similar succeeds and round-trips through
  assign_tag** (tests.rs). Specifically uncovered:
  - CJK happy path: e.g., `raw_tag: "日本語"` on empty TagSet → `Ok(Some(...))`.
  - Full-width space (`\u{3000}`) handling: `Tag::new("foo　bar")` —
    `str::trim()` does **not** strip ideographic space, and `FORBIDDEN_TAG_CHARS`
    (`tag.rs:3`) only lists ASCII ` `. So a full-width space would be
    accepted as part of a tag name. Spec doesn't explicitly rule on this
    but it's a behavioral surprise worth pinning.
  - Multi-byte trim semantics: `raw_tag: "　日本語　"` (full-width spaces
    bracketing). Behavior depends on Rust's `char::is_whitespace`, which
    includes U+3000 — so trim strips them. Should be tested for symmetry
    with `"   gpt   "`.
  - Detail: spec.md#test-perspectives does not list CJK observations, but
    the reviewer brief asked for them explicitly and the domain explicitly
    permits CJK. Without a test, regressions silently slip.
  - Recommendation: **bounce to phase 3 or phase 5** to add at least 2
    tests: `tp_cjk_happy_tag_normalizes_and_appends` and
    `tp_full_width_space_handling`. Then decide whether to extend
    FORBIDDEN_TAG_CHARS to include `\u{3000}` or document the gap.

#### Dead `load_calls` instrumentation (LOW)

- **LOW-1** `FakeRepo::load_count` (tests.rs:79–81) only asserted in
  TP-IC2 (line 472). No TP-LE / TP-NF / TP-N test asserts `load_count() ==
  1` (e.g., to prove load is reached and idempotent). Minor — the existing
  assertions are sufficient — but the instrumentation suggests an intent
  that isn't fully realized.
  - Recommendation: defer; not worth a bounce.

#### `NoOpBus` swallows event in production (LOW, carry-over)

- **LOW-2** `commands.rs:27–32` wires `NoOpBus` so `NoteTagsChanged` events
  never actually leave the use case in production. spec.md#io-input states
  "EventBus — domain event の **同期** 発行（in-process）" as a hard
  dependency, but the production runtime is a black hole. Same finding as
  auto-save-note review LOW; not specific to assign-tag.
  - Recommendation: track once Note Feed BC subscriber lands. Already
    documented in the source comment ("The Note Feed BC will subscribe
    here once it lands"). No bounce.

#### Sentinel-epoch NoteId reuse (LOW, carry-over)

- **LOW-3** `commands.rs:115–122` `parse_note_id` falls back to
  `Timestamp::UNIX_EPOCH → NoteId("19700101000000")` for unparseable
  input, downgrading malformed input to `NoteNotFound`. Same shape as
  auto-save-note. spec.md#oq-invalid-note-id-reuse already accepts this
  pattern; flagged for visibility only.
  - Recommendation: aligned with sibling slice; defer to global resolution.

#### Tauri `app_data_dir` panic path (LOW)

- **LOW-4** `commands.rs:79` `app.path().app_data_dir().expect("Tauri must
  resolve app_data_dir on supported platforms")` will panic the command
  handler on platforms where Tauri can't resolve the dir. Reasonable in
  Tauri context but the auto-save-note review noted the same; no
  consistency loss.
  - Recommendation: no action.

#### Positive observations {#positive}

- **LoadError vs PersistError separation** is correctly implemented and
  asserted (TP-LE1, application.rs:55–63 + 76–78 use distinct helpers).
  This directly addresses HIGH-2 from the auto-save-note review.
- **TagDiff two-variant enum** (application.rs:22–25) makes the no-op
  branch unrepresentable at type level — exactly as
  spec.md#impl-tag-diff intended.
- **Early `parse_tag` before `load_note`** (application.rs:50–53 runs
  before line 57) is verified by TP-IC2 (`load_count() == 0` on invalid
  tag). C-AT1 + workflow#notes "Note を load する前にバリデーション" is
  honored.
- **Event-on-success only**: persist failure path (application.rs:76–78
  early-returns via `?`) prevents `bus.publish` from running. TP-PE3
  explicitly verifies `event_count() == 0`.
- **Insertion-order preservation**: `Note::assign_tag` (note.rs:62–68)
  collects existing tags first then appends — TP-H1 asserts
  `["gpt", "coding"]` order. Matches `aggregates.md#note-aggregate-elements`
  TagSet "順序を保持".
- **Cross-slice GREEN** verified by running `cargo test --lib`:
  112 tests pass including all create_note, auto_save_note, copy_note_body
  tests after the shared deltas. No regression from the
  `NoteTagsChanged` variant addition or the `Note::assign_tag` method.
- **Layer hygiene**: `domain.rs` is pure (only thiserror + std types);
  `application.rs` only touches port traits; only `commands.rs` imports
  `tauri::*`. Hex boundary held.
- **Test ↔ spec traceability**: every `#[test]` has a `/// spec.md#tp-*
  TP-X` doc-comment.

### Verdict {#verdict}

**PASS with phase-7 followups.**

The implementation faithfully realizes spec.md's contract, all 112 cross-
slice tests stay GREEN, and the three structural concerns from the
auto-save-note review (LoadError separation, PersistError write-side-only,
parse-before-load) are correctly handled. No HIGH severity findings.

MED-1 (file two promised proposals) and MED-2 (aggregate no-op
`updated_at` divergence) are **debt the implementer explicitly committed
to surfacing in phase 7**; not blockers for the slice's correctness.
MED-3 (CJK coverage) is a real test gap the brief explicitly asked
about — recommend adding before close, but the existing impl is unlikely
to fail those tests, so impl-green doesn't need to re-open.

If the orchestrator wants strict adherence to "phase 6 proposals
required", MED-1 and MED-2 should be lifted into `/ori-finalize` before
the slice is marked complete. Otherwise PASS.
