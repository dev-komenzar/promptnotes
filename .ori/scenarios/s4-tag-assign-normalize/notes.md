# s4-tag-assign-normalize — Scenario implementation notes

## テスト方式: UI 駆動 {#test-approach}

Block のタグ入力 UI (`screen-1-block-tag-input`) を操作する。seed の Note A は
`wdio.conf.ts` の `onPrepare` が `TAURI_TEST_STORAGE_DIR` に投入する。

旧テストはアプリに一切触れない空テストだった（IPC invoke がコメントアウト、step2 は
assert ゼロ、テスト自身が別 tmpDir に書いたファイルを見るだけ）ため 92ms で
vacuously PASS していた。本改修で実アプリを駆動する形に置換した。

## frontmatter tags 形式 {#tag-format}

app の `parse_tags_inline` は **unquoted inline** `tags: [gpt, coding]` を期待する
(`create_note/infrastructure.rs`)。quoted (`tags: ["gpt"]`) は引用符ごと tag 名に
なってしまうため seed で使わない。

## Event 検証の方針 (NoOpBus 制約) {#event-verification-policy}

`assign_tag` の production 配線も `NoOpBus` (`assign_tag/commands.rs`) のため、
`NoteTagsChanged` は E2E で観測不可。event 担保は slice unit test に委ねる。
E2E で検証するには EventBus 実配線 + テスト用シームが必要（別 issue）。

## テストカバレッジ {#coverage}

| 検証項目 | 検証方法 |
|---|---|
| Tag 正規化 (`"  GPT  "` → `gpt`) | E2E step1（no-op 成立で間接検証） |
| 重複 no-op（TagSet 不変 / 永続化なし / updatedAt 不変） | E2E step1 |
| 新規 tag 追加（永続化 + chip 追加 + updatedAt 変化） | E2E step2 |
| 禁止文字 reject | S10 の責務（本 scenario 対象外） |
| event 発行 | slice unit test（E2E 不可・上記） |


