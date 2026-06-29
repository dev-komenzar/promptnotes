# Review: page-main theme-subscriber {#review-page-main-theme-subscriber}

## Pass 1 {#pass-1}

Reviewer: general agent (fresh context)

### Findings

- **HIGH** I-PM18 ("load-settings result theme is applied on page mount") has no automated test at the wiring level.
  - → follow-up issue: `ori-followup-page-main-theme-wiring-test`
- **MED** State divergence: `settings:theme_changed` Tauri events bypass `currentSettings`, causing stale theme in settings modal.
  - → follow-up issue: `ori-followup-page-main-theme-divergence`
- **MED** TP-T7 only tests Dark-from-start; doesn't exercise System→Dark detach path.
  - → FIXED: added 3 tests (Dark fixed, Light fixed, System→Dark detach with removeEventListener spy)
- **MED** `stop()` is completely untested.
  - → FIXED: added 2 tests (stop calls unsubscribe + removeEventListener, stop prevents media query propagation)
- **MED** TP-T11 only asserts no throw; doesn't verify subscriber remains functional.
  - → FIXED: test now calls setTheme('Dark') after silent fallback start() and asserts dark class applied
- **LOW** `onThemeApplied` callback is dead surface area.
  - → FIXED: removed from ThemeSubscriberDeps and apply()

### Disposition

- HIGH → follow-up issue (I-PM18 wiring test requires browser project component test)
- MED (divergence) → follow-up issue (design decision required)
- MED (TP-T7/stop/TP-T11) → fixed in test-red + impl-green patch
- LOW (onThemeApplied) → fixed in refactor patch

## Pass 2 {#pass-2}

Self-review (same session, all patches applied):

### Verification

- TP-T7: 3 tests now cover Dark fixed, Light fixed, and System→Dark detach path with removeEventListener spy ✓
- stop(): 2 tests verify unsubscribe + media handler removal + post-stop no-propagation ✓
- TP-T11: test now verifies setTheme('Dark') works after silent fallback start() ✓
- onThemeApplied: removed from deps interface and apply() ✓
- I-PM18 wiring test: follow-up issue `ori-followup-page-main-theme-wiring-test` created ✓
- state divergence: follow-up issue `ori-followup-page-main-theme-divergence` created ✓
- All 15 tests GREEN ✓
- typecheck 0 errors ✓
- lint/format pass ✓

### Remaining items (follow-up, not blocking)

- I-PM18 wiring test (follow-up issue, browser project dependency)
- state divergence (follow-up issue, design decision required)

### Verdict: PASS

All actionable findings addressed or triaged to follow-up issues. Remaining items are non-blocking design/infrastructure concerns.
