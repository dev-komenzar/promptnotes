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

# Review: page-main draft max-height (ori-9j3t) {#review-draft-max-height}

## Pass 1 {#draft-max-height-pass-1}

### Structural gates

- (a) PASS 14 tests (`bun run test --project client src/ui-page/page-main` → 4 files / 14 tests GREEN、所要 ~7.7s)
- (b) eslint PASS (prettier --check の失敗は main から存在する無関係ファイルで、本 slice 差分ではない)
- (c) N/A (page)

### Findings

#### HIGH

なし。

#### MEDIUM

- **M1 / tests/PageMain.draft-height.svelte.test.ts:76-92** — "長文末尾のカーソル行がエディタの可視範囲内にある" test は `caret.{top,bottom}` を **`.cm-scroller` の bounding rect** と比較しているが、これは `scrollHeight` ではなく visible layout box なので、「caret が scroller 内」であって「caret が viewport 内」であることは保証していない。実装前に GREEN だった理由は、max-h が無かった時代は scroller 自体の bounding rect が content full-height（＝たとえ root の `overflow-hidden` で viewport 外にはみ出ていても scroller の box 内）になっていたためで、"カーソル位置は常にエディタ内の可視範囲に追従する" (spec I-PM19) をちゃんと検証していない。post-impl では `max-h-[70vh] + overflow-y:auto` で scroller の box が実可視領域と一致するため偶然 pass しているが、regression 検知力は弱い。
  - 推奨修正: `caret.bottom <= window.innerHeight && caret.top >= 0` を併記して "caret が真に viewport 内" を assert する。もしくは scroller.scrollTop が末尾付近 (`scrollHeight - clientHeight - caret-margin` 以上) に動いたことを確認。
- **M2 / tests/PageMain.draft-height.svelte.test.ts:65-73** — "短い本文では最大高さまで伸びない" test の閾値 `window.innerHeight * 0.3 = 240px` は 1 行の CM editor (≈24-34px) に対して loose すぎ、`max-h-[70vh]` を外しても trivially 通る。実質「editor が強制的に 70vh 展開していない」ことを示す smoke test に留まり、"内容に応じた高さ" の regression ガードにはならない。
  - 推奨修正: 閾値を実際の 1 行高相当 (例 `< 6rem` = 96px) に締める、または `max-h-[70vh]` を外した control と比較する dynamic assertion にする。

#### LOW

- **L1 / regions/DraftRegion.svelte:125** — `sticky top-0 z-10` は親 (`PageMain` root) が `overflow-hidden flex-col` で scrolling container ではなくなったため **実質 no-op**。I-PM5「Feed 最上部に常時固定」は flex-col + Draft 非 flex-1 + Feed flex-1 overflow-y-auto という新構造だけで既に満たされており、`sticky` 指定は旧構造の遺物として誤読を招く。
  - 推奨修正: `sticky top-0 z-10` を削除、もしくは "flex 構造で top に pin している" ことを示すコメントを添える。regression なしなのでブロッカーではない。
- **L2 / regions/DraftRegion.svelte:184-186 (<style>)** — `.cm-scroller { overflow-y: auto }` は CodeMirror default styles が既に `overflow: auto` を持つため冗長。`min-height: 0` on `.cm-editor` が flex 子として縮むために本質的に必要な宣言で、こちらは正しい。
  - 推奨修正: 任意。コメントで "default 挙動の明示化" と註するか削除する。
- **L3 / tests/PageMain.draft-height.svelte.test.ts** — spec の 4 観点のうち「Draft region 全体やページ全体はスクロールしない」は暗黙にしかカバーされていない (test 4 の submit 可視性からの間接)。`document.documentElement.scrollTop === 0 && document.documentElement.scrollHeight <= window.innerHeight` や `region-draft` 自身の `scrollTop === 0 && scrollHeight === clientHeight` を assert すると spec invariant を直接保護できる。
  - 推奨修正: 1 ケース追加。pass には不要。
- **L4 / spec.md:122 (I-PM19)** — "本文エディタのみが縮む" を保証するためには "タグ入力 + submit 行は shrink-0" + "body は flex で縮む" という構造条件が必要だが、spec text はその責務が CSS 側にあることまでは踏み込まない。impl 側は `shrink-0` × 2 で正しく実現しているが、万一将来 body の `max-h-[70vh]` だけ残して flex-col を外されると viewport < 70vh 時に invariant が崩れる。暗黙契約が明示されていないのが唯一の懸念。
  - 推奨修正: 任意。spec I-PM19 に "タグ行と submit 行は非縮小・body のみ縮小" を 1 行追加するか、impl 側の `<style>` コメントに等価の "I-PM19 contract: tag/submit rows are shrink-0" を追記して暗黙契約を見える化。

### 退行リスク評価

- **I-PM5 (Draft 常時固定)**: PASS。構造 (非 flex-1 Draft + flex-1 Feed + root overflow-hidden) で既に実現。sticky は no-op だが副作用なし。
- **Cmd+N focus (`focusDraft`)**: PASS。`view?.focus()` 経路に変更なし。
- **Cmd+Enter (submit keymap)**: PASS。CodeMirror state 構築経路に変更なし。
- **Dark mode**: PASS。`dark:` prefix 残存、新規色指定なし。
- **I-PM6 unique-points 3 / Block 全文表示**: PASS。変更は DraftRegion に閉じ、Block 側に max-h リークなし (impl diff が FeedRegion / blocks に触れていないことを確認)。
- **Shortcut hint 表示 (ori-aqsq での直近追加)**: PASS。`shortcutHint` span は submit 行内で保持されており DOM 構造・CSS 変更の影響なし。

### 総合判定

**PASS**

理由:
1. spec I-PM19 の核 (70vh 上限 / editor-internal scroll / Draft region 自体はスクロールしない / 低 viewport で tag + submit 可視) は impl / test で満たされている。
2. 既存不変条件 (I-PM5 sticky、I-PM6、Cmd+N / Cmd+Enter、dark mode) に regression なし。structural gates も 14 tests GREEN。
3. MEDIUM 2 件は「test の regression 検知力の弱さ」であり spec 実装の正しさとは独立。post-impl の挙動自体は正しいため blocker ではなく follow-up として扱える。M1 (caret viewport 判定の強化) と M2 (短本文閾値の tightening) は次 pass で test を強化すると丸ごと解決するので、/ori-finalize 時に follow-up issue 化を推奨する。
4. LOW 4 件はいずれも cosmetic / nice-to-have で、将来の構造変更時の safeguard として spec・impl に 1 行の "暗黙契約の明示" を入れるのが望ましい。

### Disposition {#draft-max-height-pass-1-disposition}

verdict=PASS。軽微な指摘はこの PR 内で対応した (Pass 2 は不要):

- M1 → 対応: カーソル可視テストに `scroller.scrollTop > 0` と caret の viewport 内判定を追加。旧実装で RED になることを確認
- M2 → 対応: 短文テストの閾値を `< 96px` に締めた
- L2 → 対応: `.cm-scroller { overflow-y: auto }` を削除 (CodeMirror の `overflow-x: auto` により overflow-y は auto として計算される)
- L3 → 対応: 長文テストに `document.documentElement.scrollHeight <= innerHeight` を追加
- L1 → 見送り: `sticky top-0` は既存実装で無害。今回の範囲外
- L4 → 見送り: I-PM19 に「タグ行と submit 行は常に viewport 内、本文エディタのみが縮む」を既に明記済み
