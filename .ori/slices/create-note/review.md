# Review: create-note {#review-create-note}

## Pass 1 {#pass-1}

Reviewer: claude-opus-4-7[1m] (ori-reviewer, fresh-context)

Confirmed prior to review:
- `direnv exec . cargo test --manifest-path apps/promptnotes/src-tauri/Cargo.toml --lib note_capture` → **18 passed, 0 failed**
- `direnv exec . cargo clippy --manifest-path apps/promptnotes/src-tauri/Cargo.toml --all-targets -- -W clippy::all` → **0 warnings**

GREEN is genuine. The findings below concern semantics, coverage, and invariant scope — not compilation / lint.

### Findings {#findings}

- **HIGH** invariant I-N6: `Tag::new` drops the "trim" half of normalization, contradicting domain `lowercase + trim` rule
  - `apps/promptnotes/src-tauri/src/note_capture/shared/types/tag.rs:21-29`. The forbidden-char check runs on `raw` (no trim), and the stored value is `raw.to_lowercase()` — **never trimmed**.
  - Domain: `.ori/domain/aggregates.md:35` (`name: String — 正規化済み（lowercase + trim, CJK 許容）`) and `bounded-contexts.md:38` (`lowercase + trim 正規化`). I-N6 explicitly says "正規化規則（lowercase + trim、禁止文字排除）を必ず満たす".
  - Observable bug: a user typing `"Coding "` (trailing space) gets `InvalidTag::InvalidChar` instead of normalized `"coding"`. By the domain contract this should succeed and produce a Tag of `"coding"`.
  - The main-session note in the brief justifies this as "needed to make TP-I3 pass". That is backwards: TP-I3 was written from the spec's `raw_tag` wording, but the spec test perspective is about *containing forbidden chars internally*, not about *surrounding whitespace counting as a forbidden char*. TP-I3 should generate raw tags whose internal content contains a forbidden char, after the would-be trim. The current implementation makes the domain contract more restrictive than what `aggregates.md` actually says.
  - Suggested fix: `propose` (open a `/ori-propose` to either (a) update `aggregates.md` to drop "trim" from the normalization rule, or (b) fix `Tag::new` to trim before forbidden-char check and store trimmed value, and tighten TP-I3 accordingly). This is a real domain ↔ impl drift that is *not* currently tracked in `status.yaml#dirty` or `spec.md#open-questions`.

- **HIGH** panic on a legitimate (if unusual) user input — `NoteBody` error swallowed by `expect`
  - `apps/promptnotes/src-tauri/src/note_capture/slices/create_note/application.rs:36-37` calls `NoteBody::new(cmd.raw_body).expect(...)`.
  - `NoteBody::new` (`shared/types/note_body.rs:11-16`) rejects any body whose lines (after `trim_end`) equal `---`. A user typing a horizontal rule (`---` on its own line, which is **valid Markdown**) makes the Tauri command panic.
  - `spec.md#io-errors` lists only `InvalidTag` and `PersistError`, so this error path is unhandled rather than rejected by design. The main session's hint #3 explicitly acknowledges the gap.
  - This is shippable-bug territory: the slice currently turns a benign Markdown input into a process crash. The drift exists in two directions at once — (i) domain says `NoteBody` is "任意の UTF-8 文字列（空文字も許容、frontmatter 由来の `---` を含まない）" so a domain-level rejection is fine, but (ii) the spec command surface needs to expose it as a typed `Err`, not a panic.
  - Suggested fix: either `test-red` + `impl-green` to add a `CreateNoteError::InvalidBody { reason: NoteBodyError }` variant (with a TP-IB test), **or** relax `NoteBody` to escape `---` lines at the storage layer (per impl-frontmatter free area). Either way, `expect` must go. Track as a separate finding distinct from the empty-body domain drift — this one is silent.

- **MED** test coverage gap: cross-case dedupe not exercised (`["GPT", "gpt"]` → 1 tag)
  - `spec.md#tp-with-tags` covers TP-T1 (case normalize) and TP-T2 (same-case dedupe) but not their composition. The slice's behavior depends on the *order* of `to_lowercase` and dedupe in `TagSet::from_tags`. Today `Tag::new` lowercases before TagSet sees the names, so `["GPT","gpt"]` would correctly collapse to one, but there is no regression net for someone refactoring `TagSet::from_tags` to compare raw `Tag` rather than `Tag::name`.
  - Spec language ("重複排除後 1 件（I-N5、先勝ち）") implies normalization-then-dedupe, so this is the canonical happy-path edge.
  - Suggested fix: `test-red` (add a TP-T3 variant for cross-case dedupe), then propose adding it to `spec.md#tp-with-tags`.

- **MED** test coverage gap: empty-string tag (`raw_tags: vec!["".into()]`) is silently uncovered
  - `Tag::new("")` returns `TagError::Empty` (`shared/types/tag.rs:26-28`), which becomes `CreateNoteError::InvalidTag { raw: "", source: Empty }`. The screen-1 UI cross-tag-error message ("カンマ・ブラケット・空白") is misleading for that case — UI must distinguish via the inner `TagError`. No test exercises this path.
  - Worth covering because in real input, an empty tag is more likely than a comma-bearing tag (the user clears a tag input then hits Enter).
  - Suggested fix: `test-red` (TP-IT5) + decide whether `Empty` should still surface as `InvalidTag` or get its own outer variant; that decision belongs to the spec.

- **MED** test does not actually verify the write that its name promises
  - `tests.rs:103-123` (`tp_h1_happy_path_creates_note_writes_md_and_emits_event`) asserts the returned Note, but `repo` is moved into the use case and there is no peek-back at `FakeRepo::write_count`. The "writes md" half of the test name is uncovered here; only TP-PE1 (negative path) and TP-C1 (collision) check `write_count`.
  - Suggested fix: `refactor` (re-use the `Rc<FakeRepo>` pattern already present in TP-IT3/TP-PE1 to assert `write_count == 1` in TP-H1).

- **MED** infrastructure has zero tests — `FsNoteRepository` frontmatter format is unverified
  - `apps/promptnotes/src-tauri/src/note_capture/slices/create_note/infrastructure.rs:24-49` constructs the entire YAML frontmatter via `format!` / `push_str` and `fs::write`. No test exercises the resulting file content. `spec.md#impl-frontmatter` explicitly says "詳細フォーマット（日時表現、escape 規則）は infrastructure テストで固定する" — that test does not exist.
  - Observable risks not currently caught:
    - body without trailing newline → file ends mid-content (UX nit but a real format decision).
    - tag names containing YAML-significant chars that are *not* forbidden by `Tag::new` (e.g. `:`, `#`, `&`, `'`, `"`, `*`, `?`) end up unquoted inside `tags: [a:b]`, breaking YAML round-trip when a future `read_note` slice arrives.
    - tags-inline `join(", ")` is fine for the writer but bakes a format that downstream parsers must agree on.
  - Suggested fix: `test-red` to add at least one happy-path infrastructure test (write Note → assert exact file bytes including trailing newline policy) before this slice is treated as shippable. Spec already promises it.

- **MED** property test `tp_i3` does not cover what its label claims
  - `tests.rs:486-504` generates `prefix in "[a-z]{0,5}"` and `suffix in "[a-z]{0,5}"`. Combined with the forbidden char, the input space is `[a-z]*<forbidden>[a-z]*`. Uppercase, CJK, and the *surrounding* whitespace cases (which the main session said is the reason `Tag::new` checks raw, not trimmed) are *not* exercised.
  - In particular the comment in `tag.rs:18-20` ("surrounding whitespace counts as a violation too") is asserted nowhere in the property test — the property test only verifies an *interior* forbidden char.
  - Suggested fix: `test-red` to broaden the proptest strategy (uppercase A-Z, CJK range, leading/trailing whitespace) so that whatever decision is made on the I-N6 trim question is locked in by tests rather than by an inline comment.

- **LOW** zenkaku-space (`\u{3000}`) is correctly handled in body-empty guard, not tested
  - `application.rs:27` uses `cmd.raw_body.trim().is_empty()`. Rust's `str::trim` strips all Unicode `White_Space`, so `"\u{3000}"` → empty. Behavior is right, but the spec brief explicitly asked about it and no TP-E covers it. Adding a `tp_e5_zenkaku_only_is_noop` test removes ambiguity for future contributors who might "optimize" the guard to `is_ascii_whitespace`.
  - Suggested fix: `refactor` (one-line test).

- **LOW** `NoteBody::new` rejects only `trim_end`-stripped `---`, not lines with leading whitespace
  - `shared/types/note_body.rs:12` does `l.trim_end() == "---"`. A line like `"  ---"` is *not* rejected, but YAML frontmatter parsers also treat it as a delimiter. Minor; depends on how strict the eventual parser is.
  - Suggested fix: `none` until a future read-note slice forces the issue; if changed, fix together with the `expect` panic above.

- **LOW** `Tag` storage skips trim AND `Note` exposes no mutators
  - With the trim issue (HIGH #1), the only saving grace is that no one can construct a `Tag` with leading/trailing whitespace today (the raw check rejects it). So the *stored* state never violates I-N6's "禁止文字排除". The stored state *does* however violate "正規化済み (lowercase + trim)" because no trim step exists — but it does so vacuously (nothing reachable carries surrounding whitespace). The invariant holds by side-effect, not by construction. A future change that removes the raw forbidden-char check (e.g. to allow whitespace, as the trim semantics would suggest) would break this. The slice is brittle to its own resolution of HIGH #1.
  - Suggested fix: tied to HIGH #1 resolution.

- **LOW** `make_usecase` helper returns `(usecase, ())` — useless `()`
  - `tests.rs:95-98`. Refactor leftover. Cosmetic.
  - Suggested fix: `refactor`.

- **LOW** `tp_it3_invalid_tag_does_not_persist` has dead first half
  - `tests.rs:264-308` constructs a usecase, calls it, then *discards the result and rebuilds* via the `Rc<FakeRepo>` pattern. The long comment explaining the dead pattern is also misleading: the actual assertion is fine; the dead code just makes the test 2× longer than needed.
  - Suggested fix: `refactor`.

- **LOW** slice has no `commands.rs` / no Tauri-specta surface
  - Architecture (`.ori/architecture.md` Layout (Rust)) lists `commands.rs` as part of every slice's standard files. Brief #4 acknowledges this deferral. From a strict reading of the layout the slice is *not* yet shippable end-to-end (UI cannot invoke `CreateNoteUseCase`). Acceptable as long as a follow-up bd issue exists.
  - Suggested fix: `none` if a tracking issue exists (verify in finalize); otherwise file one.

- **LOW** `lib.rs` does not wire the slice into `tauri::Builder`
  - `apps/promptnotes/src-tauri/src/lib.rs:5-22` builds Tauri without registering any command. Consistent with there being no `commands.rs`, but means even integration-level smoke (manual `pnpm tauri dev`) cannot exercise the slice. Same disposition as the previous finding.

- **LOW** infrastructure exposes `FsNoteRepository::new(PathBuf)` with no validation
  - No invariant that `storage_dir` is absolute (cf. `I-S1` on the Settings side), no creation in the constructor — `fs::create_dir_all` happens inside `write`. Per spec this is fine (storage dir resolution is composition-root concern), but worth documenting that `FsNoteRepository::new` accepts a relative path silently.
  - Suggested fix: `none` for this slice; flag for the (future) settings/storage slice.

- **LOW** `tags_inline` YAML escape risk for `Tag` names containing `:` / `#` / `"` etc.
  - Already called out under the "infrastructure has zero tests" finding above; reiterated here because it's specifically an *invariant leak from VO to format*. `Tag::new` allows characters that break YAML inline lists. Either widen `FORBIDDEN_TAG_CHARS` (domain change) or quote tags on serialization (infra change). Spec leaves this open.

### Coverage matrix vs spec test-perspectives {#coverage}

| Spec ID | Implemented test | Status |
|---|---|---|
| TP-H1 (happy + event) | `tp_h1_happy_path_*`, `tp_h1_emitted_event_payload_*` | covered (write-count not asserted in main TP-H1, see MED finding) |
| TP-E1 (empty `""`) | `tp_e1_empty_body_*` | covered |
| TP-E2 (`"   "`) | `tp_e2_whitespace_only_spaces_*` | covered |
| TP-E3 (`"\n\t  \n"`) | `tp_e3_whitespace_only_mixed_*` | covered |
| TP-E4 (`"a"`) | `tp_e4_single_char_body_*` | covered |
| TP-T1 (case normalize) | `tp_t1_tags_are_normalized_*` | covered |
| TP-T2 (same-case dedupe) | `tp_t2_duplicate_tags_*` | covered |
| TP-IT1 (comma) | `tp_it1_*` | covered |
| TP-IT2 (internal space) | `tp_it2_*` | covered |
| TP-IT3 (no write on InvalidTag) | `tp_it3_*` | covered (dead first half — refactor LOW) |
| TP-IT4 (no event on InvalidTag) | `tp_it4_*` | covered |
| TP-PE1 (write IO error → PersistError, path correct) | `tp_pe1_*` | covered |
| TP-PE2 (no event on PersistError) | `tp_pe2_*` | covered |
| TP-I1 (id ↔ createdAt format roundtrip) | `tp_i1_note_id_roundtrips_*` | covered |
| TP-I2 (updatedAt >= createdAt) | `tp_i2_*` | covered |
| TP-I3 (forbidden char always rejected) | `tp_i3_*` | covered narrowly (see MED — uppercase / CJK / surrounding-ws not in proptest strategy) |
| TP-C1 (same now, empty 2nd) | `tp_c1_*` | covered |
| TP-C2 (no test by design) | n/a | n/a |

Additional missing tests (uncovered cases worth adding):
- empty-string tag in `raw_tags`
- cross-case dedupe `["GPT","gpt"]`
- zenkaku-only body `"\u{3000}"`
- `FsNoteRepository::write` actual file output (frontmatter format fixture)
- `NoteBody` frontmatter-delimiter rejection path — currently panics rather than being tested

### Verdict {#verdict}

- **NEEDS_FIX**

- Rationale: The 18 GREEN tests honestly cover every spec-listed TP, and clippy is clean. The architecture layout (`shared/types`, slice-internal layers, no cross-slice imports, ports as traits) is faithfully realized. However, two issues are not yet at "shippable Pass 1" quality: (1) `Tag::new` silently drops the "trim" half of the I-N6 normalization rule by rejecting raw-whitespace tags outright — this is a **domain ↔ impl drift that is not currently captured in `status.yaml#dirty` or `spec.md#open-questions`**, and the only argument given for it (passing TP-I3) reflects a test that itself wasn't strict enough about what it was testing; (2) `application.rs` panics via `expect` on a benign Markdown input (a body containing `---` on its own line), which is a real crash path in the Tauri command surface and is acknowledged-but-unaddressed in the brief. Fixing both is a small `propose` + `test-red` + `impl-green` cycle; until then the slice has known sharp edges. The MED-tier gaps (cross-case dedupe, empty-tag, infrastructure file-format test) should be triaged in finalize but are not individually blocking.

## Pass 2 {#pass-2}

Reviewer: claude-opus-4-7[1m] (ori-reviewer, fresh-context Pass 2)

Confirmed:
- `cargo test --lib note_capture` → **21 passed, 0 failed** (was 18; +3 TP-IB tests)
- `cargo clippy --all-targets -- -W clippy::all` → **0 warnings**

### HIGH-fix verification {#pass-2-high-fix-verification}

- **HIGH 1 (I-N6 trim drift) — FIXED.**
  - `tag.rs:17-31` now executes `raw.trim()` → empty check on `trimmed` → forbidden-char check on `trimmed.chars()` → `Self(trimmed.to_lowercase())`. The stored value is the trimmed lowercase content, so `Tag::new("Coding ")` now correctly yields `"coding"` instead of `InvalidChar`. Both halves of `lowercase + trim` per `aggregates.md#note-aggregate-elements` are realized by construction.
  - The forbidden-char check runs *after* trim, so surrounding whitespace cannot trigger a false `InvalidChar`. Interior `\t` / `\n` / `,` / `[` / `]` / ` ` still reject as required by I-N6.
  - TP-I3 (`tests.rs:560-576`) now generates `prefix in "[a-z]{1,5}"` and `suffix in "[a-z]{1,5}"` — both **non-empty**, which guarantees the forbidden character is interior to the trimmed content (cannot be stripped by `raw.trim()` even when the forbidden char is whitespace). Strategy genuinely exercises the post-trim interior forbidden-char invariant.
  - `spec.md#tp-invariants` TP-I3 (line 171) explicitly states the new contract: "trim 後の Tag content に禁止文字が **内部** 出現する raw_tag は常に reject される". The pre-trim whitespace-only case is correctly carved out to "正規化対象". Spec, test, and impl now agree.

- **HIGH 2 (NoteBody panic via expect) — FIXED.**
  - `domain.rs:19-23` adds `CreateNoteError::InvalidBody { source: NoteBodyError }` with proper `#[source]` chaining and `thiserror` display.
  - `application.rs:33-34` replaces `.expect(...)` with `NoteBody::new(cmd.raw_body).map_err(|source| CreateNoteError::InvalidBody { source })?`. Grepped the file: no `expect` / `unwrap` / `panic` on any `NoteBody` path. Pipeline now totally panic-free for valid `String` input (Step 0 → guard, Step 1 → typed Err, Step 2+ unreachable on InvalidBody).
  - TP-IB1 (`tests.rs:337-351`) sends `raw_body: "---"` and asserts `Err(CreateNoteError::InvalidBody { .. })`. With `cmd.raw_body.trim()` non-empty (`---` survives trim), Step 0 guard does not fire — Step 1 is genuinely exercised. Test claims match behavior.
  - TP-IB2 (`tests.rs:353-367`) sends `"hello\n---\nworld"` — middle `---` line — and asserts InvalidBody. `NoteBody::new` iterates `raw.lines()` and matches the interior `---` line via `trim_end() == "---"`. Test claims match behavior.
  - TP-IB3 (`tests.rs:369-403`) uses the `Rc<FakeRepo>` / `Rc<FakeBus>` peek-back pattern and asserts `write_count == 0` and `event_count == 0` after the InvalidBody path. Genuinely verifies the no-side-effect contract (C-CN4 extended to InvalidBody).
  - `spec.md#io-errors:90` documents the `InvalidBody` variant with reference to `aggregates.md#note-aggregate-elements`. `spec.md#tp-invalid-body:156-160` enumerates TP-IB1/IB2/IB3. `spec.md#oq-notebody-validation-surface:251-256` correctly tracks the upstream proposal that needs to be filed at finalize ("/ori-propose で domain/workflows/create-note.md#errors に `InvalidBody` を追加する proposal を立てる"). Drift is no longer hidden — it is now in `open-questions`.

### New findings (Pass 2 only) {#pass-2-new-findings}

None. The Pass 1 patches are surgical and do not introduce new HIGH-tier issues:
- The `Tag::new` rewrite preserves the rejection set for interior forbidden chars; `tp_it1` / `tp_it2` (existing tests for comma and internal space) still pass without modification, confirming no regression in the negative path.
- The `InvalidBody` variant is additive on `CreateNoteError` — existing match sites in tests (`InvalidTag { .. }`, `PersistError { .. }`) continue to compile and pass. No exhaustiveness break.
- Clippy clean across `--all-targets`, including the new test bodies.

(Carried over from Pass 1, deferred to phase 7 finalize triage per brief: cross-case dedupe TP-T3, empty-string tag TP-IT5, TP-H1 missing `write_count==1`, FsNoteRepository zero tests, all LOW items. Not re-listed here.)

### Verdict {#pass-2-verdict}

- **PASS**
- Rationale: Both HIGH findings are genuinely fixed at impl + test + spec levels simultaneously. (1) `Tag::new` now satisfies I-N6 "lowercase + trim + 禁止文字排除" by construction, and TP-I3's strategy (`{1,5}` prefix/suffix) actually enforces "interior" semantics rather than relying on a comment. (2) `application.rs` is panic-free; `CreateNoteError::InvalidBody` propagates via `?` with `#[source]` chaining; TP-IB1/IB2/IB3 cover the error value, the trigger inputs, and the no-side-effect contract; the upstream domain proposal is tracked in `open-questions` instead of being silently dropped. No new HIGH-tier issues were introduced by the patches. The MED/LOW items from Pass 1 remain valid carry-over work for phase 7 finalize but do not block this slice from progressing.
