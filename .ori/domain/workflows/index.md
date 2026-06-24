---
ori:
  node_id: workflow:index
  type: workflow
  depends_on:
    - aggregate:collection
    - event:collection
    - scenario:collection
---

# Workflows Index {#workflows-index}

PromptNotes の 13 workflow を BC 別に列挙。各 workflow は DMMF pipeline 形式で
別ファイルに記述（review しやすさのため）。

<!-- ori:auto-table:start -->

## Summary {#summary}

| id | trigger | output event | aggregate | BC |
|----|---------|--------------|-----------|-----|
| [create-note](create-note.md) | Cmd+Enter | NoteCreated | Note | Note Capture |
| [auto-save-note](auto-save-note.md) | 500ms debounce | NoteBodyEdited | Note | Note Capture |
| [flush-note](flush-note.md) | focus 喪失 / blur / quit | NoteBodyEdited | Note | Note Capture |
| [assign-tag](assign-tag.md) | tag 入力確定 | NoteTagsChanged | Note | Note Capture |
| [remove-tag](remove-tag.md) | × クリック | NoteTagsChanged | Note | Note Capture |
| [delete-note](delete-note.md) | 削除ボタン | NoteDeletedToTrash | Note | Note Capture |
| [restore-deleted-note](restore-deleted-note.md) | Undo ボタン | NoteRestoredFromTrash | Note | Note Capture |
| [copy-note-body](copy-note-body.md) | コピーボタン | (なし) | Note | Note Capture |
| [update-feed-filter](update-feed-filter.md) | 検索/期間/タグ操作 | (なし) | NoteFeed | Note Feed |
| [change-sort-order](change-sort-order.md) | ソートトグル | SortPreferenceChanged | NoteFeed + Settings | Note Feed |
| [update-settings](update-settings.md) | 設定モーダル保存 | StorageDirChanged / ThemeChanged | Settings | User Preferences |
| [load-settings](load-settings.md) | アプリ起動時 | (なし) | Settings | User Preferences |
| [check-for-updates](check-for-updates.md) | アプリ起動時 | NewVersionDetected (条件付) | UpdateChannel | Update Distribution |

<!-- ori:auto-table:end -->

## Coverage Matrix {#coverage-matrix}

Phase 7 validation scenarios の workflow 対応：

| Scenario | Workflow(s) |
|--|--|
| S1 (Cmd+Enter で作成) | create-note |
| S2 (AutoSave) | auto-save-note |
| S3 (Flush on blur) | flush-note |
| S4 (タグ正規化) | assign-tag |
| S5 (削除 → Undo) | delete-note → restore-deleted-note |
| S6 (連続削除でトースト置換) | delete-note ×2 |
| S7 (トースト消失後 Undo no-op) | restore-deleted-note の UI 層 guard |
| S8 (検索正規化) | update-feed-filter |
| S9 (同一 body 冪等) | auto-save-note の前段 |
| S10 (禁止文字 Tag reject) | assign-tag |
| S11 (storage_dir 変更) | update-settings |
| S12 (起動時 filter リセット / sort 復元) | load-settings |
| S13 (quit 時連続 Flush) | flush-note ×N |
| S14 (更新失敗 silent) | check-for-updates |
| S15 (同一秒内編集) | auto-save-note / flush-note |

## Open Questions {#open-questions}

Phase 9 時点で未決事項はない。

- Phase 10 (types) で各 workflow の Input / Output / Error を Rust 型として確定する
- Phase 11a (ui-fields) で各 workflow の trigger UI と error 表示位置を確定する
