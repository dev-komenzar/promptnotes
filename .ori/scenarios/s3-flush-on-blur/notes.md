# s3-flush-on-blur — Scenario implementation notes

## Flush トリガーの scope {#trigger-scope}

本 scenario は **BlockBlur**（別ブロック click による focus 喪失）のみを検証する。
`flush-note` workflow は 3 種のトリガーを共有するが、他は別 scope:

| trigger | 検証 |
|---|---|
| `BlockBlur` | 本 scenario (s3) |
| `WindowBlur` | 別 scenario（未 scaffold） |
| `AppQuit` | S13 quit-flush（未 scaffold） |

実装経路: `Block.svelte` の `$effect` が `blockState` の EDITING → 非EDITING 遷移を検知し
`runFlush('block_blur')` を呼ぶ（`cancelDebounce()` → `flush-note` slice）。

## Event 検証の方針 (NoOpBus 制約) {#event-verification-policy}

`flush-note` の production 配線も `NoOpBus`
(`note_capture/slices/flush_note/commands.rs`) のため、`NoteBodyEdited` は E2E で観測不可。
event 担保は slice の unit test (`flush_note/tests.rs`) に委ねる。E2E で検証するには
EventBus 実配線 + テスト用シームが必要（別 issue）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| focus 喪失で debounce を待たず即時 Flush | E2E step1 (`editedAt` から <500ms で永続化) |
| `.md` 永続化 (body + updatedAt) | E2E step1 |
| debounce timer キャンセル（重複保存なし） | E2E step1（flush 後 800ms 待っても updatedAt 不変） |
| body 不変なら no-op | E2E step2 |
| 同時 EDITING は高々 1 (I-PM10) | E2E step3 |
| event 発行 | slice unit test（E2E 不可・上記） |

## spec test-point 4 の扱い {#test-point-4}

spec.md#test-points の「2 ブロック同時 EDITING 時は…」は I-PM10（同時 EDITING は高々 1）と
矛盾する。step3 では I-PM10 不変条件として検証しており、spec 側の記述修正は proposal 候補。


