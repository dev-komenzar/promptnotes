---
target: domain/ui-fields/page-groups.md#widget-update-toast
by: pages/widget-update-toast
reason: I-U3 改訂 (起動時 + 24h 定期確認) の連動更新から page-groups.md と bounded-contexts.md の「起動時」記述が漏れている (ori-m2ra.4)
created: 2026-10-25
status: accepted
accepted_at: 2026-10-25
accepted_by: human (takuya.kometan@gmail.com)
---

# Proposal: page-groups.md / bounded-contexts.md の「起動時のみ」記述を I-U3 改訂に追随させる

## 発見の経緯 {#context}

- 検出元：ori-m2ra.4 の実装フェーズ（widget-update-toast / check-for-updates の再 derive）
- 2026-10-24 の proposal（`accepted/2026-10-24-check-for-updates-aggregates-update-channel-aggregate-invariants.md`）
  の連動表 `#proposal-ripple` に page-groups.md と bounded-contexts.md が含まれておらず、
  I-U3 改訂後も「起動時」だけを前提にした記述が残っている

## 現状仕様 {#current}

> domain/ui-fields/page-groups.md#widget-update-toast より：
>
> - **mount trigger**: アプリ起動時の `NewVersionDetected` event 受信
> - **unmount trigger**: dismiss / view-release のいずれかをユーザがクリック

> domain/bounded-contexts.md#update-distribution-purpose より：
>
> PromptNotes 側のロジックは「起動時に確認する」「通知する」「現在のバージョンを提示する」のみ。

## 矛盾／欠落 {#gap}

- aggregates.md I-U3 / screen-3.md `#cross-display-timing` は「起動時 + 24 時間ごと」に改訂済み。
  page-groups.md の mount trigger は「アプリ起動時」の event に限定しており、定期確認由来の event と食い違う
- page-groups.md の unmount trigger「view-release」は screen-3.md `#cross-dismiss-view`
  （view-release 後も Toast は残す）と以前から食い違っている
- bounded-contexts.md の「起動時に確認する」も I-U3 改訂後は不正確

派生側（widget-update-toast spec）は screen-3.md / aggregates.md を正として derive した（view-release で Toast は残す、
定期確認由来の event も表示する）。挙動上の問題は無いが、上流文書の食い違いが残る。

## 提案する変更 {#proposal}

```markdown
- **mount trigger**: `NewVersionDetected` event 受信（起動時および 24 時間ごとの定期確認。I-U3 / I-U4）
- **unmount trigger**: dismiss をユーザがクリック（view-release では残る。screen-3 cross-dismiss-view）
```

bounded-contexts.md#update-distribution-purpose:

```markdown
PromptNotes 側のロジックは「起動時と 24 時間ごとに確認する」「通知する」「現在のバージョンを提示する」のみ。
```

## 代替案 {#alternatives}

- **A. reject**：派生側は screen-3.md / aggregates.md を正として実装済みのため、文言の食い違いを許容する
