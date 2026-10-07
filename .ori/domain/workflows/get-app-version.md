---
ori:
  node_id: workflow:get-app-version
  type: workflow
  depends_on:
    - aggregate:UpdateChannel
---

# get-app-version {#get-app-version}

設定モーダル表示時に、ビルド時に埋め込まれた現在のアプリバージョン
（`UpdateChannel.current_version`, I-U1）を返す読み取り専用 query。
ネットワークアクセスは行わない。

## Input {#input}

```rust
struct GetAppVersionQuery;    // 入力なし（ビルド時定数を読むだけ）
```

## Output {#output}

- `AppVersion` — 表示用のバージョン文字列
  - 通常: `Version` の Display（例: `0.2.2`）
  - fallback: ビルド時定数が `Version` VO として parse できない場合
    （pre-release / build metadata 付き等）は **raw 文字列をそのまま** 返す
- domain event: **なし**（状態を変更しない query のため）

## Errors {#errors}

- なし（parse 失敗は raw 文字列 fallback で吸収する。表示専用のため reject しない）

## Steps {#steps}

1. `readBuildVersion: () → String`
   - ビルド時に埋め込まれた app version 文字列を取得（I-U1: immutable）
2. `toAppVersion: String → AppVersion`
   - `Version::from_str` 成功 → 正規化された `Version` の Display
   - 失敗 → raw 文字列

## Dependencies {#dependencies}

- なし（ビルド時定数のみ。`UpdaterPort` / `EventBus` は使わない）

## Notes {#notes}

- `check-for-updates` と **同じビルド時定数** を情報源とする
  （両者の `current_version` が食い違ってはならない）
- 呼び出しは widget-settings-modal の mount 時。値は不変なので結果をキャッシュしてよい
- I-U3（起動時 1 回のみの更新確認）とは無関係: 本 query は GitHub Releases に問い合わせない
- Update Distribution BC から他 BC への参照は発生しない（UI 層のみが購読、
  [context-map](../context-map.md#decisions-update-distribution) 参照）
