# change-sort-order — Implementation notes

## 唯一の逆流 (NoteFeed → Settings) を application service で 1 トランザクション化

`aggregates.md#notes-sort-side-effect` で警告された Customer-Supplier の唯一の逆流。13 workflow 中で **NoteFeed と Settings を同時に touch する唯一の slice**。本 slice 経由でのみ `NoteFeed.sort == Settings.sort_preference` の同期不変条件が保たれる。

## PersistError 再利用 (`UpdateSettingsError::PersistError`)

新しい error enum を作らず、`pub use crate::user_preferences::slices::update_settings::domain::UpdateSettingsError as ChangeSortOrderError;` で型エイリアス採用。理由:

- UI 層が両 slice からの「Settings 書き出し失敗」を同一 handler で扱える
- 「PersistError は update-settings の path を再利用する」ユーザ指示の literal 解釈
- TP-E4 で fn-pointer coercion により compile-time に「同一型」を pin

副作用: `UpdateSettingsError::InvalidPath` variant も type 上は本 slice の error type に含まれるが、`execute` 経路で生成し得ない（validation を呼ばないため）。spec.md#oq-error-type-extraction に follow-up として記録。

## no-op 判定は Settings 側を見る (feed.sort は見ない)

`current_settings.sort_preference() == cmd.new_sort` のみで no-op 判定。`feed.sort` は比較対象に含めない。

理由: Settings が Customer-Supplier の Supplier (source of truth) であり、本 slice が **唯一の同期経路** なので `feed.sort != Settings.sort_preference` の状態は workflow 上発生し得ない。仮に乖離した状態で本 slice を呼んだ場合、no-op path では `feed` が返るだけで矯正は行われない（防衛的に矯正しない設計）。

→ spec の TP-noop 観点に「乖離ケースは workflow 上 invariant により発生しない」を含意。reviewer 観察事項として記録。

## review 指摘の対応 (phase 6 → phase 7 へ送られた fix)

- (4) 乖離防衛 test の有無 → spec / 本 notes.md に「workflow invariant で発生し得ない」を明示（test 追加は scope 外、防衛対象が無いため）
- (7) update-feed-filter/notes.md に「superseded by change-sort-order」追記 → 完了
