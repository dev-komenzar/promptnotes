# s6-delete-replace — Scenario implementation notes

## Event 検証の方針 (NoOpBus 制約) {#event-verification-policy}

`delete_note` / `restore_deleted_note` の production 配線も `NoOpBus`
(`delete_note/commands.rs`, `restore_deleted_note/commands.rs`) のため、
`NoteDeletedToTrash(A/B)` → `NoteRestoredFromTrash(A/B)` の**発行順序は E2E で観測不可**。
担保は各 slice の unit test。E2E は UI + FS 状態変化で間接的にカバーする（s5 と同方針）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| A 削除後のファイル不在（OS ゴミ箱へ移動） | E2E (`existsSync` false) |
| 連続削除で A の Toast が置換されない（2 件維持） | E2E (toast count = 2 + 両 `data-toast-id` 存在) |
| 積み上げ順（最新 B が画面上側） | E2E (`getBoundingClientRect().top`: B < A) |
| A の Undo が B に影響しない（per-toast 独立） | E2E (A のみ復元 / B の toast 残存 / B の `.md` は absent のまま) |
| A / B のファイル復元（body=`alpha` / `bravo`） | E2E (`existsSync` true + `readFileSync`) |
| NoteFeed 表示更新（削除で -1/-2 / Undo で復元） | E2E (block count) |
| トーストスタックが空になる | E2E (toast count = 0) |
| event 発行順序 | slice unit test（E2E 不可・上記） |
| Toast 有効期間超過後の no-op | 別 scenario S7 (`s7-undo-after-toast`) の責務 |

## production 修正 {#production-fix}

初回 E2E は「最新トーストが最下段」で RED。`ToastRegion.svelte` のトーストスタックが
`flex-col-reverse` だったため、`store.entries`（新しい順 = 先頭）が反転描画され
`screen-1.md#cross-toast-display`（最新が画面上側）に違反していた。`flex-col` に修正。
store の順序規約（先頭 = 最新）・`entries[0]` を undo する `undoLatest` は不変。
