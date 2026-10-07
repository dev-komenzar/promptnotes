# Review: widget-settings-modal {#review-widget-settings-modal}

## Pass 1 {#pass-1}

### Structural gates {#structural-gates}

- (a) boundary test: PASS (`bun run test` 178/178、store.test.ts 15/15)
- (b) arch lint:     PASS (`svelte-check` 0 errors、eslint / prettier clean)
- (c) public_entry:  PASS (`ui-widget/settings-modal/` から `@tauri-apps/api/core|app` の直接 import 0 件)

### Semantic findings (reviewer: ori-reviewer, fresh context) {#semantic-findings}

- I-SM9 / I-SM10 / I-SM11 と tp-sm-app-version-* は実装・テスト両面で trace 済み。既存 I-SM1..8 の経路に変更なし（purely additive）
- **LOW** `WidgetSettingsModal.svelte`: 表示条件が `{#if store.appVersion}` の truthy 判定で、空文字も非表示になる。slice 契約 (C-GAV1) 上は実害なし / 推奨: 将来 `!== null` に揃える
- **LOW** `store.svelte.ts`: `onPreviewTheme` dep が spec#io の注入点列挙に無い。component 内部 seam のため影響なし
- **INFO (pre-existing, out of scope)** inline error 文言が spec「絶対パスを指定してください」と impl `Path must be absolute.` で乖離 → 別 issue で追跡

### Disposition {#disposition}

- LOW 2 件: 差し戻し不要（許容）
- INFO: 本 diff と無関係のため別 issue 化

### Verdict {#verdict}

verdict: PASS
