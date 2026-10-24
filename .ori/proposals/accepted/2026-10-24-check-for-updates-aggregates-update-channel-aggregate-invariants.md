---
target: domain/aggregates.md#update-channel-aggregate-invariants
by: slices/check-for-updates
reason: I-U3「確認は起動時 1 回のみ」では、アプリを終了せず使い続ける利用者 (特に macOS) が長期間更新に気付かない (ori-m2ra.4)
created: 2026-10-24
status: accepted
accepted_at: 2026-10-24
accepted_by: human (takuya.kometan@gmail.com)
---

# Proposal: I-U3 を「起動時 + 24 時間ごとの定期確認」に改訂し、同一バージョンの重複通知を抑止する

## 発見の経緯 {#context}

- 検出元：bug epic `ori-m2ra`（v0.2.2 で v0.2.3 の更新通知が出ない）の調査。child issue `ori-m2ra.4`
- 起動時配線の欠落 (`ori-m2ra.1`) を直しても、I-U3 のとおり起動時 1 回しか確認しないと、
  アプリを終了せず使い続ける利用者は再起動まで新バージョンに気付かない
  - macOS ではウィンドウを閉じてもプロセスが残ることが多く、数週間再起動しないこともある
- 他アプリの例：Sparkle は既定 1 日 1 回（最短 1 時間）、VS Code は起動後 + 1 時間ごと、Chrome は数時間おき
- Tauri v2 updater plugin にはスケジュール機能が無い。いつ呼ぶかはアプリ側で決める必要がある
- endpoint は `github.com/.../releases/latest/download/latest.json`（REST API ではない）。1 日 1 回程度なら rate limit は問題にならない

## 現状仕様 {#current}

> domain/aggregates.md#update-channel-aggregate-invariants より：
>
> - **I-U3**: 確認は **アプリ起動時 1 回のみ**。常駐 polling はしない
>   （spec の core 動作: 通知のみ）

> domain/domain-events.md#new-version-detected-trigger より：
>
> `UpdateChannel::check_at_startup()` 成功 かつ `latest_release: Some(_)` の場合。
> 起動時 1 回のみ発行される（I-U3）。

> domain/workflows/check-for-updates.md#error-handling より：
>
> - リトライ・常駐 polling は行わない（I-U3）

> domain/ui-fields/screen-3.md#cross-display-timing / #cross-dismiss-view より：
>
> - アプリ起動時 1 回のみ `check-for-updates` 実行（I-U3）
> - `screen-3-dismiss`: Toast を閉じるだけ。次回アプリ起動時に再度 `check-for-updates`
>   が走り、まだ新バージョンがあれば再表示

## 矛盾／欠落 {#gap}

- 求めている挙動：起動したままのアプリでも、リリースから 1 日程度で更新に気付けること
- 現状の I-U3 は起動時以外の確認を明示的に禁じているため、定期確認を入れると不変条件違反になる
- 定期確認を入れると、同じバージョンの toast が毎回出る問題が新たに起きる。現状ドメインには
  「どのバージョンを通知済みか」の概念が無い

## 提案する変更 {#proposal}

### 1. I-U3 の改訂 (aggregates.md#update-channel-aggregate-invariants) {#proposal-i-u3}

```markdown
- **I-U3**: 確認は **アプリ起動時に 1 回** 行い、以降は **前回確認から 24 時間経過するごとに**
  再確認する。前回確認の時刻は揮発（プロセス内のみ、永続化しない）。
  失敗時のリトライ（短い間隔での再試行）はしない。次の確認は通常の 24 時間周期を待つ
- **I-U4**（新設）: 同一プロセス内で一度通知した `Release::version` 以下のバージョンについては、
  `NewVersionDetected` を再発行しない（通知済みバージョンは揮発。再起動後は再び通知される）
```

- 24 時間は Sparkle の既定値に合わせた値。間隔は定数（設定 UI は作らない）
- sleep / 休止から復帰した時に timer が遅れる可能性がある。実装側では「単調時計 (monotonic clock) で
  経過時間を測る」または「ウィンドウ focus 復帰時に経過時間を確認し、24h を超えていれば確認する」
  のどちらかで吸収する（実装詳細なので domain には書かない）

### 2. 操作名の改訂 (aggregates.md#update-channel-aggregate-commands) {#proposal-operation}

- `UpdateChannel::check_at_startup()` → `UpdateChannel::check()`
  （起動時専用ではなくなるため。signature・silent failure の扱いは変えない）
- I-U4 を守る責務は application service が持つ（通知済みバージョンを揮発 state で保持する）。
  aggregate は今と同じく 1 回分の確認結果を返すだけ

### 3. 連動する文書の更新 {#proposal-ripple}

| 文書 | 箇所 | 変更 |
| --- | --- | --- |
| domain-events.md | `#new-version-detected-trigger` | 「起動時 1 回のみ発行」→「起動時および定期確認で、未通知のバージョンを検出した時に発行（I-U3 / I-U4）」 |
| domain-events.md | `#new-version-detected-timing` | 「silent failure、I-U3 補足」の参照先を S14 に寄せる |
| workflows/check-for-updates.md | 概要 / `#error-handling` | 「起動時に」→「起動時および 24h ごとに」、「常駐 polling は行わない」→「失敗時の即時リトライはしない。次回は通常周期」 |
| validation.md | `#s14-update-check-failure` | Given「アプリ起動直後」→「起動時または定期確認時」。When を `check()` に改名 |
| validation.md | 新 scenario（例: `#s-update-same-version-once`） | Given 通知済み v0.2.3 / When 24h 後の定期確認で再び v0.2.3 を検出 / Then `NewVersionDetected` は発行されない |
| ui-fields/screen-3.md | `#cross-display-timing` | 「起動時 1 回のみ」→「起動時 + 24h ごと（I-U3）。同一バージョンは 1 プロセス 1 回（I-U4）」 |
| ui-fields/screen-3.md | `#cross-dismiss-view` | dismiss 後は「次回起動時」または「それより新しいバージョンが定期確認で見つかった時」に再表示 |
| ui-fields/screen-3.md | `#cross-suppression` | 変更なし（永続的な「このバージョンを skip」は引き続き MVP 範囲外） |

### 4. 派生側への影響 {#proposal-impact}

- `slices/check-for-updates`：spec 再 derive。起動時配線（`ori-m2ra.1`）の仕組みに 24h 周期と
  通知済みバージョンの state を足す。tests に I-U4 の検証を追加
- `pages/widget-update-toast`：I-UT1「常駐 polling は行わない」の文言を直す（frontend は listen するだけなので
  挙動は変わらない見込み。I-UT3「最新 event で上書き」は今後実際に起きうる）
- `scenarios/s14-update-check-failure`：Given の文言変更に追随
- `slices/get-app-version`：I-U3 を引用しているだけなので文言追随のみ

## 代替案 {#alternatives}

- **A. ウィンドウ focus 復帰時のみ確認**（前回から 24h 以上経過していれば）
  - timer が要らない。ただしバックグラウンドで常に開いているウィンドウ（focus が動かない）では確認されない
- **B. 通知済みバージョンを永続化**（Settings に `notified_version` を保存）
  - 再起動後も同じ toast を出さない。ただし `#cross-suppression` で MVP 範囲外とした内容に踏み込む。
    「再起動のたびに再表示」という今の dismiss 仕様とも衝突する
- **C. 現状維持（reject）**
  - 起動時 1 回のまま。`ori-m2ra.4` を close し、長時間起動したままの利用者が更新に気付かない問題は受け入れる
