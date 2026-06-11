---
name: ori-derive
description: /ori-flow phase 1。manifest の derives_from とドメイン文書から slice spec.md を合成する
---

ユーザが `/ori-derive <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 1 として起動した際に、**該当 slice の `spec.md` をドメイン文書から派生**します。**派生のみ。実装は phase 4 の責務**。

## 引数

- `slice-id`：対象 slice の id（`.ori/slices/<id>/` が存在する事を前提）

## 役割

- **派生器**：`manifest.yaml` の `derives_from:` に列挙されたドメイン section を読み、slice 単位の spec に再構成
- **整合性チェッカー**：複数 upstream に矛盾があれば停止し、`/ori-propose` を促す
- **記録係**：spec.md は **derived** ファイル。`coherence.source: derived` で書き、人間が直接編集すると `/ori-sync --force` が要求される

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/manifest.yaml`（必須。`derives_from:` を持つ）
  - `manifest.derives_from` に列挙されたドメイン section（例：`domain/aggregates.md#note-aggregate`）
- 出力：`.ori/slices/<id>/spec.md`
  - frontmatter: `coherence.source: derived`、`upstream:` に派生元 section を列挙、`hash:` に派生元のスナップショットハッシュ
  - 必須 H2 セクション（**`.apm/instructions/feature-spec.instructions` 準拠** — instructions ファイル名は legacy のまま）

## 必須セクション（feature-spec.instructions 準拠）

| H2 | id | 内容 |
|----|----|------|
| `## 概要 {#overview}` | overview | この slice が解く問題、対応する workflow / UI |
| `## 入出力 {#io}` | io | 入力（command / form）と出力（event / view）の型 |
| `## 不変条件 {#invariants}` | invariants | slice 完了時に成り立つ条件（domain 不変条件 + slice 固有制約） |
| `## テスト観点 {#test-perspectives}` | test-perspectives | テストで検証すべきシナリオ列挙（phase 3 で test 化） |
| `## 実装ノート {#impl-notes}` | impl-notes | アーキ層への落とし込みヒント（依存 interface 等） |

各 H2 は `{#id}` 必須。H3 を追加する場合も `{#id}` 必須。

## 手順

1. **slice 存在確認**：
   ```bash
   bash scripts/check-slice-exists.sh <slice-id>
   ```
   - exit 0: 存在 → 次のステップへ
   - exit 2: 類似候補あり → ユーザに「これですか？」と確認、Yes なら正しい id で再開
   - exit 1: 未発見 → 新規 slice 作成を**ユーザに確認**してから進める
2. **manifest.yaml の読み込み**：`.ori/slices/<id>/manifest.yaml` を Read。`derives_from:` が空ならエラー停止し「先に DDD phase で domain を整備するか、manifest に upstream を追記してください」と案内
3. **upstream section の取得**：
   ```bash
   bash scripts/resolve-upstream.sh <slice-id>
   ```
   派生元 section のパスとハッシュを取得
4. **矛盾検出**：複数 upstream が同じ概念について異なる規定を持つ場合、停止して `/ori-propose` を促す（自動マージしない）
5. **spec.md の synthesis**：
   - 上記 5 セクションを必須として埋める
   - 不明な事項は **推測で埋めず** `**TBD**` マーカーを残し、後段で人間に問う
   - 上流 section の文言を引用する際は `> domain/aggregates.md#note-aggregate より:` の出典を残す
6. **spec.md の自己検証**：
   - 必須 H2 5 種が揃っているか（`## 概要`、`## 入出力`、`## 不変条件`、`## テスト観点`、`## 実装ノート`）
   - 全 H2/H3 に `{#id}` があるか（grep: `^###? [^{]+$`）
   - frontmatter `coherence.source: derived` と `upstream:` の有無
7. 検証失敗時は **1 回だけ** 自動修正を試み、それでも失敗ならユーザに判断を委ねる
8. **beads issue 更新**：
   ```bash
   bd update ori-derive-<slice-id> --status=closed --notes="spec.md generated from <N> upstream sections"
   ```

## 出力テンプレート

```markdown
---
coherence:
  source: derived
  last_derived: 2026-05-14
  upstream:
    - domain/aggregates.md#note-aggregate
    - domain/workflows/capture-auto-save.md
  hash:
    domain/aggregates.md#note-aggregate: a1b2c3...
    domain/workflows/capture-auto-save.md: d4e5f6...
---

# capture-auto-save spec {#capture-auto-save-spec}

## 概要 {#overview}

ユーザの入力中に Note を自動保存する slice。空白のみの本文は破棄。

> domain/workflows/capture-auto-save.md より：trigger は user types、output は NoteSaved event

## 入出力 {#io}

- Input: `CaptureAutoSaveCommand { noteId: NoteId, body: NoteBody, occurredAt: Instant }`
- Output: `NoteSaved` event（非同期）
- Error: `EmptyBody`, `NoteNotFound`

## 不変条件 {#invariants}

- `body` 編集時 `updatedAt` は前回より大（domain/aggregates.md#note-aggregate より）
- 空白のみの body は永続化されず破棄

## テスト観点 {#test-perspectives}

- happy path: 通常テキスト → NoteSaved event
- empty body: 空白のみ → 永続化されない
- non-existent note: 不明 noteId → NoteNotFound
- timestamp monotonic: 連続編集で updatedAt が常に増分

## 実装ノート {#impl-notes}

- 依存: `NoteRepository`, `Clock`
- throttle 間隔は **TBD**（spec で確定 / domain で確定を後段で問う）
```

## 注意

- **自動 scaffold は禁止**：slice が存在しなくても勝手に新規作成を呼ばない（ユーザ確認必須）
- **spec.md は派生ファイル**：直接編集には `/ori-sync --force` が必要
- **推測で埋めない**：`TBD` を残し、人間判断に委ねる箇所を明示
- このスキルは test や impl を書かない。**phase 1 = spec 派生のみ**

## 次のアクション

phase 1 完了後、`/ori-flow` 内部なら自動的に phase 2 へ。単独呼び出しの場合：

- **メインパス**：`/ori-plan <slice-id>` — phase 2。下流 phase（test-red / impl-green / refactor / review）の beads issue description を埋める
- **TBD を解消するパス**：`/ori-distill phase=<関連 phase>` でドメインに戻り合意形成 → 再度 `/ori-derive`
- **矛盾発見パス**：`/ori-propose` で upstream 修正提案を作成 → 人間レビュー
