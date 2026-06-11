---
name: ori-doctor
description: ori プロジェクトの健康診断。.ori/ を歩き schema / status.yaml ⇔ beads / cross-reference 整合性を検査。報告のみで自動修復はしない
---

ユーザが `/ori-doctor` を呼んだ際、**ori プロジェクト全体の健康状態を診断**します。**report-only**：自動修復はしない（domain は人間判断、code は他 phase の責務）。

## 役割

- **検査官**：schema 違反、dirty 残存、cross-reference 切れ、status.yaml と beads の同期ズレを検出
- **レポーター**：問題を行番号付きで列挙、優先度を付ける
- **修復方針案内人**：各問題に対して「どのスキル / コマンドで直すか」を提示

## 入力 / 出力

- 入力：プロジェクトルート（`.ori/` がある場所）
- 出力：標準出力に diagnostic report。**ファイル生成しない**（`/ori-doctor` 自体は副作用なし）

## 検査項目

### 1. ドメイン文書の schema 健全性

- `.ori/domain/` 配下の全ファイルを Read し手動検証
- 全 H2/H3 に `{#id}` があるか
- frontmatter `coherence:` ブロックがあるか
- 必須セクション（slice / page / phase ごと）の有無

### 2. 派生文書の hash 一致

- `.ori/slices/*/status.yaml` の `upstream_hash` と現在の domain section ハッシュを比較
- 不一致なら **dirty 残存** として報告

### 3. status.yaml ⇔ beads の同期

- 各 slice について `status.yaml.phase_status` と `bd show ori-<phase>-<id>` の `status` を突き合わせる
- ズレを報告（例：beads では closed だが status.yaml では in_progress）

### 4. Cross-reference の整合

- spec.md / workflows/<id>.md / screen-N.md の `upstream:` 列挙先がすべて実在するか
- 存在しない section へのリンクを broken-link として報告

### 5. proposal の残存

- `.ori/proposals/` 配下の pending proposal をカウント
- N > 0 なら `/ori-review-proposals` を案内

### 6. orphan slice / page / domain section

- どの slice / page からも `derives_from:` されていない domain section（dead documentation の可能性）
- どの domain にも対応しない slice / page（孤立 slice / page）

### 7. beads 健全性

- `bd doctor` を呼び出し結果を取り込む
- `bd orphans` で参照切れ issue

## 手順

1. **`.ori/` 存在確認**：なければ「`/ori-init` で初期化してください」と返す
2. **スクリプトで全検査を実行**：
   ```bash
   bash .apm/skills/ori-doctor/scripts/run-checks.sh
   ```
   個別検査は以下で構成：
   - `check-domain-schema.sh` — ドメイン文書の frontmatter + anchor 検証
   - `check-slice-schema.sh` — slice の manifest/status ファイル存在確認
   - `check-hash-consistency.sh` — 派生ファイルの upstream 参照実在確認
   - `check-cross-ref.sh` — derives_from / upstream の cross-reference 検証
   - `check-proposals.sh` — pending proposal カウント
   - `lint.js` — `.ori/` の Markdown anchor / id 規約検証（JS）：
     ```bash
     node .apm/skills/ori-doctor/scripts/lint.js [<path>] [--strict]
     ```
3. **結果を集約**してレポートを生成
4. **報告 only**：自動修復は行わない

## レポートフォーマット

```
🩺 ori-doctor report

═══ Domain Schema ═══
✓ .ori/domain/discovery.md
✗ .ori/domain/aggregates.md:42 — H2 "Note Aggregate" missing {#id}
  fix: edit aggregates.md, add anchor manually (human judgment)

═══ Hash Consistency ═══
⚠ slices/capture-auto-save: 1 upstream out of sync
  upstream: domain/aggregates.md#note-aggregate
  fix: /ori-flow capture-auto-save (re-derive)

═══ Status Sync ═══
✓ all slices / pages in sync with beads

═══ Cross-Reference ═══
✗ slices/edit-past-note-start/spec.md → broken link: domain/aggregates.md#draft-aggregate
  fix: target was renamed to #note-aggregate; edit manifest.derives_from

═══ Proposals ═══
ℹ 2 pending proposals
  /ori-review-proposals

═══ Orphans ═══
⚠ domain/aggregates.md#tag-aggregate — derived by no slice / page
ℹ this may be intentional (read-only reference)

═══ Beads ═══
✓ bd doctor: all green

═══ Summary ═══
✗ 2 errors  ⚠ 2 warnings  ℹ 2 info
recommended action: fix broken cross-ref first (blocks /ori-flow on edit-past-note-start)
```

## 注意

- **自動修復しない**：domain 文書の手入れ・spec の再 derive はそれぞれ別スキル
- **read-only**：このスキル自体は何もファイルを変更しない（副作用ゼロ）
- **CI 統合可能**：将来 `/ori-doctor --json` 相当の出力をパイプして CI gate に使う想定

## 次のアクション

レポート内容に応じて以下を案内：

- **schema 違反パス**：`vim .ori/domain/<file>.md` で手動修正（自動修正しない）→ 再度 `/ori-doctor`
- **hash 不一致パス**：`/ori-flow <id>` で該当 slice / page を再 derive
- **broken cross-ref パス**：該当 slice / page の `manifest.yaml` を更新 or 旧 anchor を domain 側で復活
- **proposal 残存パス**：`/ori-review-proposals` で人間判断
- **orphan domain パス**：意図的なら無視、不要なら削除を検討
- **beads 不整合パス**：`bd dolt push` / `bd dolt pull` で再同期、`bd orphans` で個別対処
- **全部 green パス**：`/ori-feature-status` で次の作業候補を選ぶ
