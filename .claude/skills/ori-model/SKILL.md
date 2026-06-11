---
name: ori-model
description: capability-role × phase × agent から具体 model への割当をユーザと対話で決める
---

ユーザが `/ori-model` を呼んだ際、**現在の model 割当を表示し、必要に応じて変更**します。ori の capability-role モデル（capability = reasoning / deep / fast 等の役割名、それを具体 model に解決）と phase 別 / agent 別 override を扱います。

## 役割

- **状態表示者**：現在の割当を見やすく整形
- **意図翻訳者**：「review をもっと安く」「fast を deepseek にしたい」などの自然言語を config 変更に翻訳
- **影響説明者**：変更前に「これでコストは N 倍になる / レビュー精度が低下する」等を説明

## capability-role モデル（前提）

| capability | 用途 |
|-----------|------|
| reasoning | 設計・レビュー（複雑な推論） |
| deep | 派生・実装（コード生成） |
| fast | refactor・整理（規則的変換） |

各 capability は `.apm/agents/` の config で具体 model に解決される。例：

```
reasoning → claude-opus-4-7
deep      → claude-sonnet-4-6
fast      → claude-haiku-4-5
```

phase / agent 単位の override も可能：

```
phase=review override: capability=reasoning + model=o1-pro
agent=ori-reviewer override: capability=reasoning
```

## 手順

### 表示モード

ユーザが「現状を見せて」と言ったら：

```bash
node .apm/skills/ori-model/scripts/show.js
```

を実行し、capability × phase の model 割当を整形提示。

### 変更モード

ユーザが「変えたい」と言ったら、まず **意図** をヒアリング：

1. **対象スコープ**：
   - 全体（capability の解決先 model を変える）
   - 特定 phase（例：review だけ別 model）
   - 特定 agent（例：ori-reviewer 専用 model）
2. **方向性**：
   - cost down（安い model に下げる）— トレードオフ：精度低下
   - quality up（より強力な model）— トレードオフ：コスト増
   - vendor 切替（Anthropic → DeepSeek 等）
3. **config ファイルを編集**：agent の frontmatter（`name`, `model`）を更新
4. **影響の説明**：
   - phase 別コスト見積を簡易表示
   - 「review を haiku に下げると adversarial 視点が弱くなる」等の警告
5. **ユーザ承認後に file write**
6. **設定確認**：agent の frontmatter を再読み込みして変更を確認

## 出力例

```
現在の model 割当：

  capability    model
  ────────────────────────────────────
  reasoning     claude-opus-4-7
  deep          claude-sonnet-4-6
  fast          claude-haiku-4-5

  phase override:
    review → capability=reasoning (no override)

  agent override:
    ori-reviewer → capability=reasoning

どこを変更しますか？
  [1] capability の解決先 model
  [2] 特定 phase に model を固定
  [3] 特定 agent に model を固定
  [4] 表示のみ（変更しない）
```

## よくある依頼パターン

| ユーザ依頼 | 操作内容 |
|----------|---------|
| 「review を deepseek にしたい」 | `ori-reviewer` agent の model を `deepseek-v4-pro` に更新 |
| 「全部 fast を haiku に統一」 | `fast` capability の解決先を `claude-haiku-4-5` に |
| 「コスト 30% 削減したい」 | refactor / impl-green を一段下げる案を提案 |
| 「reviewer を別 vendor にしたい（adversarial 多様化）」 | `ori-reviewer` agent の model を他社 reasoning model に変更 |

## 注意

- **変更前に必ず confirm**：cost / quality のトレードオフを明示
- **このスキルは workflow を回さない**：設定のみ
- **vendor 切替時は API key を別途設定**：`.env` 編集はユーザに委ねる（スキルは指示のみ）
- **APM 配布でのデフォルト**：プロジェクト初期化時の値を尊重。大きな変更は記録（commit で残す）

## 次のアクション

設定変更後、ユーザに以下を提示：

- **試運転パス**：`/ori-flow <small-slice>` で小さい slice を 1 つ回し、コスト・品質を確認
- **元に戻すパス**：`git diff` で agent ファイルの変更を確認。問題あれば `git checkout HEAD -- .apm/agents/`
- **doctor 確認パス**：`/ori-doctor` で全体に影響が出ていないか
- **commit 推奨**：model 設定は session 跨ぎで重要。`git add .apm/agents && git commit` を案内
