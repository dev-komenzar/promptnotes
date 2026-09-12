# s2-autosave-debounce — Scenario implementation notes

## Event 検証の方針 (NoOpBus 制約) {#event-verification-policy}

`NoteBodyEdited` の E2E 検証は現状の production 配線では不可能。

- production の `auto_save_note` command は `NoOpBus` を注入している
  (`apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/commands.rs`)。
  発行された `NoteBodyEdited` はどこにも配線されず、フロント / DB / ログから観測できない。
- そのため spec.md#test-points の「event 発行」は scenario (E2E) の検証対象から外し、
  slice の unit test で担保する:
  `apps/promptnotes/src-tauri/src/note_capture/slices/auto_save_note/tests.rs`
  (`DomainEvent::NoteBodyEdited` の発行を assert)。

### E2E で event を検証したくなった場合 {#event-verification-followup}

production の EventBus を実配線に差し替え、テスト時に捕捉可能なシーム
(例: `TAURI_TEST_EVENT_LOG` 等) を設ける必要がある。これは scenario ではなく
note-capture BC / page-main 側の設計変更なので、必要になった時点で別 issue /
proposal として扱う。

## テストカバレッジ (spec#test-points との対応) {#coverage}

| spec test-point | 検証方法 |
|---|---|
| auto-save 発火タイミング 500ms± | E2E test 1 (`waitUntil` でファイル変化検知 + elapsed 400–3000ms) |
| ファイル更新 | E2E test 1 (body 内容) |
| updatedAt 更新 | E2E test 1 (frontmatter updatedAt の変化) |
| event 発行 | slice unit test (`auto_save_note/tests.rs`) — E2E 不可（上記） |
| NoteFeed updatedAt sort 反映 | store unit test (`stores/feed.test.ts` の applyAutoSave 並び替え) |
| 冪等性 (S9) | E2E test 2 (body を元に戻す編集試行 → 永続化されない) |

## debounce 値 {#debounce}

domain (`domain/workflows/auto-save-note.md` 500ms / README 500ms) に合わせ、
`Block.svelte` の `autoSaveDebounceMs` デフォルトを 600ms → 500ms に修正済み。

