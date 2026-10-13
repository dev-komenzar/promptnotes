---
target: domain/ui-fields/screen-1.md#cross-filter-immediate
by: pages/page-main
reason: 検索入力欄の表示値と IME 変換中の filter 送信について規定が無く、正規化値の書き戻しで日本語 IME 入力が壊れた (ori-fdhh)
created: 2026-10-13
status: pending
---

# Proposal: 検索入力欄の表示値は raw 入力とし、IME 変換中は filter を送信しない

## 発見の経緯 {#context}

- 検出元：`pages/page-main` の bug triage (`/ori-bug`, bd `ori-fdhh`) → impl-green
- Toolbar の検索フォーム (`screen-1-toolbar-search-query`) に IME で「かな」と入力すると「kかかnかなかな」になる
- 原因：`ToolbarRegion.svelte` が backend から返る `filter.query` (NFKC + lowercase + trim 済み) を入力欄に書き戻していた。変換中の `input.value` が外から書き換わって IME が壊れる。IME 以外でも、大文字が小文字に、末尾空白が trim される
- domain / page spec のどちらにも「入力欄に何を表示するか」「IME 変換中に filter を送るか」の規定が無い。そのため、正規化値を表示値として echo する実装でも spec 違反にならなかった

## 現状仕様 {#current}

> domain/ui-fields/screen-1.md#fields-toolbar より：
>
> | `{#screen-1-toolbar-search-query}` | 検索 | `String → Option<NormalizedQuery>` | - | text input (icon 付) | `Cmd+F` で focus。1 文字入力ごと即時 filter |

> domain/ui-fields/screen-1.md#cross-filter-immediate より：
>
> - 検索文字列 / 期間 / タグ filter の変更は **debounce なしで即時 filter**
>   （ローカル完結なので遅延不要、Q7）
> - 結果 0 件のときは「該当する Note がありません」を Feed 領域に表示

> pages/page-main/spec.md#invariants-cross-region より：
>
> - **I-PM11（filter / sort の即時反映）**: Toolbar の filter / sort 変更は debounce なしで Feed に反映。

## 矛盾／欠落 {#gap}

- 型 `String → Option<NormalizedQuery>` は一方向の変換を示している。しかし入力欄の表示値が raw `String` か正規化値かは明記されていない
- 「1 文字入力ごと即時 filter」が IME 変換中の未確定文字にも適用されるのか、規定が無い
- 未確定文字で filter を送ると、応答の echo や再描画が composition を壊しやすい。日本語 (Core user の主言語) で検索が使えなくなる

## 提案する変更 {#proposal}

`domain/ui-fields/screen-1.md#cross-filter-immediate` に次を追加する。派生する `pages/page-main/spec.md#invariants-cross-region` にも invariant (例: I-PM20) として反映する。

- **検索入力欄の表示値**: ユーザーが入力した raw 文字列をそのまま表示する。NFKC / lowercase / trim は `NormalizedQuery` (filter 条件) 側にだけ適用し、正規化値で入力欄を上書きしない
- **IME 変換中の扱い**: `compositionstart`〜`compositionend` の間は filter を送信しない。確定時 (`compositionend`) に確定値で 1 回送信する。`compositionend` と確定 `input` の発火順は WebView (WebKitGTK / WKWebView / Chromium) によって異なる。どちらの順でも送信は 1 回にする
- **ASCII など非 composition 入力**: 従来どおり 1 文字ごとに即時送信する (Q7 / I-PM11 は維持)
- page-main spec のテスト観点に `tp-search-query-input` を追加する
  1. 変換中は送信しない
  2. 確定時に 1 回だけ送る (両方の発火順)
  3. `Foo ` が入力欄に保たれる
  4. Clear all で入力欄が空になる

影響範囲：page-main (Toolbar region) のみ。`update-feed-filter` slice の C-UF1〜C-UF8 / I-F1 (正規化規則) は変更しない。

## 代替案 {#alternatives}

- domain を変えず、page-main spec だけを `/ori-sync --force` で局所修正する。ただし I-PM11 の「即時反映」の出所は ui-fields なので、IME 例外は上流に置くほうが一貫する
