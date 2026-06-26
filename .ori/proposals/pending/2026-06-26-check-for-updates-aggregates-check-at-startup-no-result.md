---
proposal:
  id: 2026-06-26-check-for-updates-aggregates-check-at-startup-no-result
  status: pending
  source: slice:check-for-updates
  created_at: 2026-06-26
  target_files:
    - .ori/domain/aggregates.md#update-channel-aggregate-operations
  related_beads:
    - ori-2lm
    - ori-2lm.9
---

# Domain 修正提案: `UpdateChannel::check_at_startup` の signature を no-Result に修正

## 背景

`aggregates.md#update-channel-aggregate-operations` (行 236) に以下の宣言がある:

```rust
- `UpdateChannel::check_at_startup() -> Result<UpdateChannel, UpdateError>`
  - async ネットワーク呼び出し。Tauri updater plugin に委譲
  - 失敗は silent（ユーザの作業を妨げない）
```

一方 `check-for-updates` slice の spec.md C-CFU1（および `workflows/check-for-updates.md#error-handling` の「全ての UpdateError を application service の outer layer で握り潰す」）は、外部呼び出し側に `UpdateError` を**伝搬させない**契約を要求している。slice 実装の `CheckForUpdatesUseCase::execute(cmd) -> UpdateChannel` は no-Result API としてこれを満たした。

aggregates.md の signature と spec の契約に **矛盾** がある:

- aggregates.md は「`Result<UpdateChannel, UpdateError>` を返す」と書いている → caller は `match` で Err 分岐できる
- spec C-CFU1 / S14 は「Result を露出しない / Err は silent」 → caller が Err 分岐すること自体が禁止

## 提案

aggregates.md 行 236 を以下に置換:

```rust
- `UpdateChannel::check_at_startup() -> UpdateChannel`
  - async ネットワーク呼び出し。Tauri updater plugin に委譲
  - 失敗は **application service 内部で silent に握り潰し**、`latest_release: None` の
    `UpdateChannel` を返す（S14 / I-U2 / `workflows/check-for-updates.md#error-handling`）。
  - 内部実装は `Result<UpdateChannel, UpdateError>` を持つ private fn を経由してよいが、
    外部 API は **`Result` を露出しない**。
```

## 影響範囲

- `check-for-updates` slice: 既に no-Result API で実装済 → 影響なし（むしろ domain doc が impl に追従する形）
- 後続 slice / wiring (ori-2lm.8, ori-6l4 後): aggregates.md の signature を直接参照する箇所が無くなり、再度 Result を露出するリスクが消える
- 他 aggregate (Note / NoteFeed / Settings) には影響なし

## 却下時の代替

aggregates.md の signature を信じて `execute` を `Result` 返却に変更する案もあるが、その場合:
- spec C-CFU1 と `workflows/#error-handling` の改訂が必要
- S14 walkthrough (silent failure) の意味が「caller が Err を受けて握り潰す」に変わる
- load-settings slice の C-LS1 (always-Ok) と一貫性が崩れる

→ 推奨しない。spec 側を採用するのが整合的。

## 受理時の追加作業

- aggregates.md の対応箇所を編集
- 派生先 slice spec の `coherence.hash:` を再計算（`/ori-derive check-for-updates` 再実行）
- domain glossary の `check_at_startup` 説明（あれば）も更新
