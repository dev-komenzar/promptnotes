# s5-delete-undo-in-window — Scenario implementation notes

## Event 検証の方針 (NoOpBus 制約) {#event-verification-policy}

`delete_note` / `restore_deleted_note` の production 配線も `NoOpBus`
(`delete_note/commands.rs`, `restore_deleted_note/commands.rs`) のため、
`NoteDeletedToTrash` → `NoteRestoredFromTrash` の**発行順序は E2E で観測不可**。
担保は各 slice の unit test。E2E は UI + FS 状態変化で間接的にカバーする。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| 削除後のファイル不在（OS ゴミ箱へ移動） | E2E (`existsSync` false) |
| Undo 後のファイル復元（body=`hello`） | E2E (`existsSync` true + `readFileSync`) |
| NoteFeed 表示更新（削除で -1 / Undo で復元） | E2E (block count) |
| toast 表示 / 非表示 | E2E (toast count) |
| トースト有効期間内の Undo | E2E（delete → pause 1s → undo） |
| event 発行順序 | slice unit test（E2E 不可・上記） |
| 有効期間超過後の no-op | 別 scenario S7 の責務 |

## 削除ボタンの操作 {#delete-button}

delete ボタンは hover 時のみ操作可能（`opacity-0` / `pointer-events-none`）。
WebKitWebDriver の `moveTo` は CSS `:hover` を安定発火しないため、テストは
`dispatchEvent(new MouseEvent('click'))` で直接 dispatch する。


