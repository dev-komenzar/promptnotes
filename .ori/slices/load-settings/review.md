# Review: load-settings {#review-load-settings}

## Pass 1 {#pass-1}

Reviewer: claude-opus-4-7[1m] (ori-reviewer, fresh-context)

Confirmed prior to review:
- `direnv exec . cargo test --manifest-path apps/promptnotes/src-tauri/Cargo.toml --lib user_preferences` → **25 passed, 0 failed**
- Test list re-counted: 25 in `user_preferences::slices::load_settings::tests` (1 of which is `_sort_order_type_is_exposed`, a no-op import guard — see MED finding below). The brief's "25 unit tests incl. 1 proptest" is accurate; effective spec-tied tests are 24.
- GREEN is genuine. The findings below concern semantics, scope, and trace fidelity — not compilation / lint.

### Findings {#findings}

- **HIGH** I-S2 check rejects exactly the case the spec text says is **allowed**
  - `apps/promptnotes/src-tauri/src/user_preferences/slices/load_settings/application.rs:76-81` (`violates_i_s2`): the predicate is `storage_dir.starts_with(config_path.parent())`. So `config_path = /a/b/c.json`, `storage_dir = /a/b/d` is flagged as a violation.
  - But `spec.md#invariants-settings-aggregate` (I-S2) explicitly states: "macOS の場合 `Application Support/promptnotes/notes` vs `Application Support/promptnotes/settings.json` は同じ親だが、ファイル `settings.json` 自身は対象ではなく `config_path.parent() == storage_dir.parent()` でも I-S2 違反ではない。**チェック対象は `storage_dir` がディレクトリとして `config_path.parent()` を含まないこと**".
  - Read literally: I-S2 should reject only the case where `config_path.parent()` itself is a descendant (or equal) of `storage_dir` (i.e. config file would live inside the storage dir, creating the circular-reference risk Q6 was about). The current impl is the **opposite direction** — it rejects when `storage_dir` is a descendant of `config_path.parent()`, which is the OS-conventional default layout (`Application Support/promptnotes/{settings.json,notes/}`).
  - Concrete fallout in `commands.rs`: at runtime `resolve_default_storage_dir` joins `app_data_dir() + "notes"`. On many platforms `app_data_dir` and `app_config_dir` share a parent — `storage_dir.starts_with(config_path.parent())` is **true** for the OS default. The use case then drops the value… and falls back via `unwrap_or_else(|| self.os_dirs.default_storage_dir())` which produces the same value (no I-S2 check applied to the default). For the **default path** the bug is invisible. But the moment a user types `/Library/Application Support/promptnotes/my-notes` (the documented allowed shape!), it is silently rewritten to the default.
  - The test `tp_i3_storage_dir_is_not_under_config_path_parent` encodes the wrong direction too: it asserts that `/tmp/promptnotes-test-config/notes` is rejected, but per the spec quoted above the same-parent case is **allowed**.
  - Suggested fix: `propose` first (the spec text and the implementation choice clearly disagree — confirm with domain which direction is correct), then `test-red` + `impl-green` to align. Most likely the predicate should be `config_path.parent().starts_with(storage_dir.as_path())` (i.e. config lives inside storage_dir).

- **HIGH** C-LS1 ("戻り値は常に有効な `Settings`、`Result` / `Option` 露出なし") is broken at the Tauri boundary by `expect()`
  - `apps/promptnotes/src-tauri/src/user_preferences/slices/load_settings/commands.rs:15-29`: both `resolve_default_storage_dir` and `resolve_config_path` use `.expect(...)` on Tauri path-resolution errors. `StorageDir::try_from(...).expect(...)` is also a panic path.
  - `spec.md#invariants-slice-specific` C-LS1 is explicit: "**`Result` / `Option` 型は API 表面に露出しない**" — and `#io-errors` reinforces "なし。本 slice は workflow 定義上「失敗しない」契約".
  - TP-AS1 only types `LoadSettingsUseCase::execute` and so does not catch this. The actual Tauri-facing surface is `#[tauri::command] async fn load_settings(...)`, which **can panic** if Tauri ever returns `Err` from `app_data_dir()` / `app_config_dir()` (which the Tauri docs allow — e.g. on a misconfigured platform / locked-down sandbox). Behaviorally this propagates as a poisoned async task to the frontend, not as a fallback to defaults.
  - The brief explicitly flags this in observation #3 as "verify whether `expect()` paths remain". They do.
  - Suggested fix: `impl-green` — replace `expect` with a `PathBuf` default derived in-process (e.g. `dirs::data_dir().unwrap_or_else(...)`, or use `OsDirs` for both `config_path` and `default_storage_dir` so the trait is the only fallible boundary, with that boundary guaranteeing a default). Add a dedicated TP for the Tauri-command surface that panics-or-not is contract-tested (or document in `#out-of-scope` that the Tauri command may panic and demote C-LS1 to "use case level only", which would be an `/ori-propose` candidate, not a silent gap).

- **HIGH** TP-AS1 does not actually check what spec.md#tp-api-shape claims
  - `apps/promptnotes/src-tauri/src/user_preferences/slices/load_settings/tests.rs:500-506` binds `fn(&LoadSettingsUseCase<RcFs, RcOs>, LoadSettingsCommand) -> Settings`. This proves the return type is `Settings` (not `Result<Settings, _>` / not `Option<Settings>`), but TP-AS1's spec text adds "シグネチャに `Result` / `Option` / **panic** を持たない". A function-pointer bind cannot prove the absence of panic — and as the HIGH finding above shows, panic paths exist transitively (via `StorageDir::try_from(...).expect` inside `commands.rs`, and via `expect` on `app.path().app_data_dir()`).
  - Combined with the brief's open question `oq-no-result-typelevel`, this is a known soft spot. The current test is fine for "no `Result`/`Option`" but oversells itself on "no panic".
  - Suggested fix: `test-red` — either narrow the spec wording (drop "panic" from TP-AS1, track separately) **or** add a runtime `std::panic::catch_unwind` smoke test against `LoadSettingsUseCase::execute` for the failure-injected rig. Then `propose` to clarify `oq-no-result-typelevel`.

- **MED** I-S2 is not actually enforced on the `OsDirs` default branch
  - `application.rs:54-55`: `.filter(|sd| !violates_i_s2(sd, config_path)).unwrap_or_else(|| self.os_dirs.default_storage_dir())`. The defaulted value is never re-checked. If a misconfigured `OsDirs` impl ever returns a path that does violate I-S2, the use case happily passes it through.
  - This is defensible (the contract on `OsDirs` is "produces a path that satisfies I-S1 & I-S2"), but it is **not stated** in `shared/ports.rs` and not enforced by the type system (`OsDirs::default_storage_dir(&self) -> StorageDir` only carries the absolute-path guarantee from the `StorageDir` smart constructor). The "OS 慣習 fallback satisfies I-S2 automatically" claim in `spec.md#invariants-settings-aggregate` is parenthetical and not codified.
  - Suggested fix: `test-red` (add a test where `OsDirs` returns a violation path and assert the slice's behavior is well-defined — either re-check or document the contract) + `impl-green` if re-check is the chosen behavior.

- **MED** `_sort_order_type_is_exposed` is a placeholder, not a spec-traced test
  - `tests.rs:544-547`. The comment admits as much. It exists only to suppress an unused-import warning.
  - This is harmless but misleading when counting "tests per TP" — it is **not** a test perspective from `spec.md#test-perspectives`, yet it shows up in the GREEN tally as if it were.
  - Suggested fix: `refactor` — either drop it (and the unused `SortOrder` import; the existing tests already touch `SortField`/`SortDirection`), or relabel it `_import_guard` so the trace is honest.

- **MED** C-LS3 vs C-LS4 boundary is implementation-correct but the spec text is internally inconsistent — please record this as an OQ resolution, not as silent agreement
  - The impl in `parse_top_level_object` returns `None` for any non-`Object` top-level (array / null / scalar / parse error). All four feed into the field-level fallback path with `obj == None`, which then yields full I-S3 defaults. So `"null"` → all defaults, `"[]"` → all defaults, parse error → all defaults. This **matches** `spec.md#tp-parse-fail` TP-P1 / TP-P2 / TP-P3 / TP-P4.
  - However `spec.md#invariants-slice-specific` C-LS3 says "**フィールド単位の部分復元はしない**、保守的" while C-LS4 says "valid JSON だが一部フィールド欠損の場合は…**欠損フィールドのみ** I-S3 で補完". The current impl uses field-level fallback **universally** — it just happens that when the top-level is not an `Object`, every field is missing, so C-LS3's "all defaults" is reproduced as an emergent consequence. C-LS3 is **not** a separate code path. This is fine semantically but the spec wording implies two distinct mechanisms.
  - The brief flags this in the special-attention observation #1, asking specifically about `"null"`. The behavior is correct, but the "C-LS3 経路で全 default、現 impl の挙動を確認" claim in the main session's summary papers over the fact that there is no distinct C-LS3 code path. If a future change adds field-level recovery from partial top-level corruption, the "C-LS3 = strict refusal" guarantee silently disappears with no test pinning it.
  - Suggested fix: `propose` — collapse C-LS3 and C-LS4 in `spec.md` into a single "JSON が Object でない場合は『全フィールド欠損』とみなす" rule (which the impl already follows), or add a TP that asserts the **structural** distinction (e.g. that a top-level `null` is rejected by `parse_top_level_object`, observable via a method exposed for testing). Currently neither test nor impl distinguishes the two cases — they just produce the same output.

- **MED** TP-PT3 / TP-I2 fix the "field-level fallback for invalid values" rule but `spec.md#open-questions` still calls it **暫定採用 / 未解決 OQ**
  - `oq-field-level-fallback`'s status line: "本 spec では暫定採用。phase 4 (impl-green) 着手前に user 確認 → 必要なら `/ori-propose` で domain 修正". The brief says: "暫定方針が test で固定されているか、仕様変更時に test が壊れる構造になっているか" — answer: the **tests do fix it** (`tp_pt3_*`, `tp_i2_*`), but neither the OQ status nor a `/ori-propose` upstream record acknowledges that the slice now has tests that will **break** if domain decides the other direction. The OQ has graduated from "暫定" to "load-bearing" without anyone noticing.
  - Same applies to `oq-no-result-typelevel`: TP-AS1 exists and is GREEN. The OQ status says "**TBD**" but the implementation has committed to the assertion-via-fn-pointer pattern.
  - Suggested fix: `propose` (open `/ori-propose` against `.ori/domain/workflows/load-settings.md#notes` for the field-level fallback rule, and against `.ori/domain/workflows/load-settings.md#errors` for the no-error contract) so the OQs close into upstream changes. Then update `spec.md#open-questions` to mark them resolved with traces to the proposals.

- **LOW** `LoadSettingsCommand` has no upstream representation outside the workflow's input section
  - `domain.rs:1-8` defines it. `spec.md#io-input` quotes the workflow exactly. Fine. But the impl re-defines it from scratch rather than importing — common pattern in the create-note slice too. Acceptable. No action.

- **LOW** Pipeline is collapsed compared to spec text
  - `spec.md#impl-pipeline` describes four steps with a `SettingsRaw` DTO. The impl skips `SettingsRaw` and goes directly from `Value` (`serde_json::Map<String, Value>`) to per-field decode via `pick_or_default`. This is cleaner and gives field-level fallback for free (the brief's intent), but the spec's step description is now stale. Worth a note in `spec.md#impl-pipeline`.
  - Suggested fix: `refactor` (docs only) — update `spec.md#impl-pipeline` to reflect the actual `Value` → `pick_or_default` flow, or label the original step list as "conceptual pipeline" and add a "実装メモ" subsection.

- **LOW** `serde_json` in `application.rs` is fine per spec but worth a comment
  - The brief asks: "domain.rs / application.rs に I/O 依存が漏れていないか". `application.rs` uses `serde_json` (pure) and **does not touch `std::fs`** — that is correctly confined to `infrastructure.rs`. No issue, but a one-line comment at the top of `application.rs` ("pure: no I/O, all side effects pass through `FileSystem`/`OsDirs` ports") would make the layer contract explicit for future readers. Optional.

- **LOW** `tp_i4_idempotent_no_double_ensure_dir_on_second_run` does the opposite of what the spec says (and asserts so)
  - `spec.md#tp-invariants` TP-I4: "**2 回目の `ensure_dir` は no-op**（C-LS8、冪等）". The test asserts `fs.ensure_count() == 2` and comments "ensure_dir is invoked each call (use case is stateless); idempotency is delegated to FileSystem impl". This is **the right design** — the use case shouldn't track idempotency state — but it directly contradicts the spec text.
  - This is the same "tests have moved past spec, spec not updated" pattern as TP-PT3.
  - Suggested fix: `propose` — update `spec.md#tp-invariants` TP-I4 to "C-LS8 idempotency is delegated to `FileSystem::ensure_dir`; use case may call it on every execute", or `refactor` the test name/comment so the contradiction is no longer asserted as if it were intentional.

### Coverage assessment {#coverage}

- TP-H1 / TP-H2 / TP-H3: covered (h3 is a constructor signature pin, fine)
- TP-A1 / TP-A2 / TP-A3: covered
- TP-P1 through TP-P4: all covered (including `"null"` per brief observation #1)
- TP-PT1 / TP-PT2 / TP-PT4: covered
- TP-PT3: covered, but OQ status stale (see MED)
- TP-M1 through TP-M4: covered. TP-M3 is a "did not panic" assertion-by-execution — acceptable.
- TP-I1: covered as proptest (1 case via `proptest::option::of(".*")`); the input space is broad enough to catch absolute-path violations
- TP-I2: covered, but OQ status stale
- TP-I3: covered, but the **direction** of the I-S2 check disagrees with spec (HIGH)
- TP-I4: asserts non-idempotency, contradicting spec (LOW)
- TP-AS1: covered narrowly — proves no `Result`/`Option`, does not prove no panic (HIGH)
- TP-AS2: covered (combined-failure smoke)
- I-S4 (out of scope): **correctly excluded**. Nothing in `application.rs` writes back to `settings.json`; the slice is read-only + ensure_dir. The brief's concern #6 — "commands.rs が暗黙に「設定変更後の再起動経路」を阻害していないか" — is fine. No write path exists yet, so the future `update-settings` slice is not blocked.

### 総合判定 {#verdict}

**NEEDS_FIX**

Differ-back ordering (severity-first):

1. `propose` — I-S2 direction (HIGH). The spec and impl genuinely disagree about which side of `starts_with` the violation lives on. Need a domain-level decision before re-writing tests. Touches `aggregates.md#settings-aggregate-invariants` (I-S2 wording) and possibly Q6's original rationale.
2. `impl-green` — remove `expect()` panic paths from `commands.rs` (HIGH). This is a real shippable-bug: a sandbox-limited environment makes the Tauri command crash instead of falling back to defaults. Can be done independently of finding #1.
3. `test-red` + `impl-green` — strengthen TP-AS1 to actually exclude panics, or trim the spec wording (HIGH). Pair with #2.
4. `test-red` — add `OsDirs`-violates-I-S2 test (or document the port contract) (MED).
5. `propose` — close `oq-field-level-fallback` and `oq-no-result-typelevel` into upstream `/ori-propose` and update `spec.md#open-questions` to point at them (MED).
6. `propose` — clarify C-LS3 vs C-LS4 (MED) and TP-I4 idempotency wording (LOW).
7. `refactor` — drop or rename `_sort_order_type_is_exposed`; update `spec.md#impl-pipeline` to reflect the `Value`-based pipeline (MED + LOW).

If `propose` rounds (#1, #5, #6) come back saying "the spec is right, fix the impl", item #1 cascades into another `impl-green`. If they come back the other way, item #1 collapses into a one-line `spec.md` edit. Either way, the I-S2 finding alone blocks PASS — silently rewriting a user's `storage_dir` choice back to the default is a user-visible regression risk that no test currently catches because the test was written to match the impl, not the spec.

## Pass 2 {#pass-2}

Reviewer: claude-opus-4-7[1m] (ori-reviewer, fresh-context, re-review)

Scope: verify only the three HIGH findings from Pass 1 (#1 I-S2 direction, #2 C-LS1 at Tauri boundary, #3 TP-AS1 overclaim). MED / LOW items 4-7, 8-10 are explicitly deferred to `/ori-finalize` → `/ori-propose` per the patch brief.

Confirmed prior to review:
- `direnv exec . cargo test --manifest-path apps/promptnotes/src-tauri/Cargo.toml --lib` → **52 passed, 0 failed**
- `direnv exec . cargo clippy --manifest-path apps/promptnotes/src-tauri/Cargo.toml --all-targets -- -D warnings` → clean
- `user_preferences::slices::load_settings::tests` re-counted: **25 tests** (was 25 with 1 placeholder; placeholder removed, TP-I3 rewritten in place, TP-I3b added → net 25, consistent with the brief)

### HIGH 1: I-S2 direction (Pass 1 → patched) {#pass-2-h1}

**Verdict: RESOLVED**

- `application.rs:74-81` `violates_i_s2` now reads `config_path.starts_with(storage_dir.as_path())`. Rationale comment ("config が storage_dir の **子孫** であるケースのみ reject する") matches `spec.md#impl-i-s2-direction` (line 228-234) verbatim in intent.
- Domain side (`.ori/domain/aggregates.md:164-165`): "Settings の永続化先 (`app_config_dir/settings.json`) は `storage_dir` 配下にしない（Q6 決定: 循環参照回避）". This reads as "the config file must not live inside the storage dir" — i.e. config is a descendant of storage_dir is the violation. Patched direction `config_path.starts_with(storage_dir)` is the formal expression of "config_path is below storage_dir", which is the documented violation. **No contradiction with `aggregates.md#settings-aggregate-invariants`.**
- `tests.rs#tp_i3_config_path_inside_storage_dir_is_rejected` (450-468): asserts that `config_path = /tmp/promptnotes-test-config/settings.json` + `storage_dir = /tmp/promptnotes-test-config` falls back to defaults. Direction pinned correctly.
- `tests.rs#tp_i3b_sibling_layout_is_allowed` (470-489): asserts the macOS-conventional `Application Support/promptnotes/{settings.json, notes/}` layout (modelled as `/tmp/promptnotes-test-config/{settings.json, notes/}`) is preserved through the use case. This is the exact case Pass 1 said the previous impl silently rewrote. **The regression risk is now pinned by test.**
- `spec.md#impl-i-s2-direction` is a new section that records the design decision in spec form. No silent agreement; the choice is documented and traceable.

### HIGH 2: C-LS1 at Tauri boundary (Pass 1 → patched) {#pass-2-h2}

**Verdict: RESOLVED with one acknowledged residual**

- `commands.rs:19-30` `resolve_default_storage_dir`: the prior `expect()` on `app.path().app_data_dir()` is replaced with `.ok().map(|p| p.join("notes")).unwrap_or_else(|| env::temp_dir().join("promptnotes/notes"))`. The Tauri-side fallible path no longer panics.
- `commands.rs:32-38` `resolve_config_path`: same shape — `env::temp_dir().join("promptnotes/settings.json")` fallback. No `expect()`.
- Residual: line 26-29 keeps `StorageDir::try_from(env::temp_dir()).expect("std::env::temp_dir() is absolute by OS contract")` as the last-sentinel. POSIX guarantees `/tmp` (or `$TMPDIR`) is absolute; Windows `GetTempPath2W` returns an absolute path. Rust's `std::env::temp_dir()` documentation does not enumerate platforms exhaustively but in practice always produces an absolute path on every supported tier-1/2 target. The inline comment is honest about the load-bearing assumption. **Best-effort panic-free is achieved**; this single residual is at the edge of what is provable without a full `Result` propagation — acceptable for a Tauri boundary that is documented out of `LoadSettingsUseCase::execute` scope.
- `spec.md#impl-tauri` (lines 220-226) now explicitly documents the `env::temp_dir()` fallback strategy and labels it "best-effort". The contract is no longer silent.
- Real shipping risk Pass 1 raised (sandbox-limited environment causing Tauri command crash instead of fallback): **mitigated** for `app_data_dir()` / `app_config_dir()` failure. The only remaining panic path is on a platform where `env::temp_dir()` itself fails to be absolute — which is not a realistic deployment scenario for Tauri targets.

### HIGH 3: TP-AS1 oversells (Pass 1 → patched) {#pass-2-h3}

**Verdict: RESOLVED**

- `spec.md#tp-api-shape` (line 156): "**panic-free は型レベルでは検証不能**のため本 TP のスコープ外。Tauri-boundary (`commands.rs`) は infrastructure 層で `env::temp_dir()` fallback により best-effort panic-free を維持する". The wording is now narrow and accurate: TP-AS1 only claims "no `Result`/`Option` in signature", which is exactly what the fn-pointer bind in `tests.rs:524-528` proves. The "panic-free" claim — which Pass 1 correctly flagged as type-impossible to prove — is gone.
- The shift to "Tauri-boundary panic-free is infra concern" does **not** weaken C-LS1's protection at the use-case level: `LoadSettingsUseCase::execute -> Settings` is still type-pinned, and TP-AS2 (combined failure injection) still asserts a `Settings` comes out under arbitrary IO failure. The boundary panic concern is now appropriately scoped to `#impl-tauri`, which is the right architectural layer for it (infra ports, not core domain).
- `tests.rs:519-528` comment was updated in step with the spec text.
- Pass 1 finding had a secondary concern: "narrow the spec wording **or** add `catch_unwind`". The brief chose the first option (narrow the spec), which is the cheaper and more honest choice — `catch_unwind` against `LoadSettingsUseCase::execute` would only have provided a smoke test, and the use case has no panic paths now that `serde_json` failures are caught (`.ok()` chains) and `ensure_dir` is wrapped in `let _ =`.

### Regression check {#pass-2-regression}

- Test count: 25 in `load_settings::tests` (was 25 in Pass 1). Net change matches brief's accounting: -1 placeholder (`_sort_order_type_is_exposed`), -0 (TP-I3 rewritten in place), +1 (TP-I3b added). `SortOrder` import line at the top of `tests.rs` is gone (verified by `grep`); clippy is clean (no unused-import warning). **No regression.**
- Total project tests: 52 passed, same shape as Pass 1's tally minus none. note_capture slice tests unchanged.
- All Pass 1 GREEN tests remain GREEN; the rewritten TP-I3 still asserts I-S2 enforcement, just from the correct direction now.

### Out of Pass 2 scope (forwarded) {#pass-2-deferred}

Per the patch brief's policy, the following Pass 1 items are **not** re-evaluated and are routed to `/ori-finalize` for `/ori-propose` triage:

- MED #4: I-S2 not enforced on `OsDirs` default branch (`application.rs:55`) — recommend `test-red` + port-contract docs
- MED #5: `oq-field-level-fallback` / `oq-no-result-typelevel` status stale — recommend `/ori-propose` against `domain/workflows/load-settings.md`
- MED #6: C-LS3 vs C-LS4 spec wording is internally inconsistent — recommend collapse via `/ori-propose`
- LOW #8: `LoadSettingsCommand` re-defined (acceptable, no action)
- LOW #9: pipeline collapsed vs `spec.md#impl-pipeline` — recommend `refactor` (docs only)
- LOW #10: TP-I4 spec wording contradicts test assertion — recommend `/ori-propose` to update spec to "idempotency delegated to FileSystem impl"

These do **not** block Pass 2 verdict.

### 総合判定 (Pass 2) {#pass-2-verdict}

**PASS**

理由:
1. The three HIGH findings from Pass 1 are all resolved. The I-S2 direction now matches both the slice spec and the domain aggregate invariant. The Tauri-boundary panic path is reduced to a single OS-contract-bound sentinel that is documented in `spec.md#impl-tauri`. TP-AS1's overclaim is replaced with a narrow, type-provable assertion plus an explicit cross-reference to where the boundary concern lives.
2. The patch did not silently relax invariants to fit the impl. Instead, where the spec and impl disagreed (HIGH 1), the patch reaffirmed the spec, fixed the impl, and added a new test (TP-I3b) to pin the previously silent allowed case. This is the right direction of fix.
3. No regression: 52/52 GREEN, clippy clean, `SortOrder` unused-import removed without breaking the build. Test count math matches the brief.
4. Deferred MED/LOW are explicitly tracked for `/ori-finalize` → `/ori-propose`. None of them are HIGH-severity bugs; deferring them respects the single-pass policy without leaving silent landmines.

Proceed to `/ori-finalize`.
