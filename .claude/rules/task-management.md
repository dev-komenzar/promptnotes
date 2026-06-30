> **根拠**: beads 公式 skill 同梱の `BOUNDARIES.md` (Pattern 1: bd as Strategic, TodoWrite as Tactical) に従う。
> `bd setup claude` が CLAUDE.md に書き込む「do NOT use TodoWrite, TaskCreate」は古い厳格 template であり、
> 同じ beads marketplace 内の BOUNDARIES.md と矛盾している。本 instruction は公式同士の整合性回復として
> tier 構造を採用する。

## Tier 構造 {#tier-structure}

```
bd epic / parent issue        (= 1 PR、bundling 単位)
  ↓
  bd child issue              (= 永続 sub-deliverable、ship 可能単位)
    ↓
    TodoWrite items           (= 1 session 内の実装 step、file 編集レベル)
```

- **bd (strategic)**: multi-session work / 依存関係 / 永続 context / side quest を track
- **TodoWrite (tactical)**: 現 session 内の linear な実行 step を track。複雑化したら bd へ promote

## Epic 化 trigger {#epic-trigger}

- **`/ori-flow` 起動時**: 即 epic 化 (feature scaffold は構造的に複数 deliverable に分解されるため)
- **その他**: 推定変更 file 数 ≥ 5 OR 推定変更 package ≥ 2 なら epic 化を提案
- **共通**: session 跨ぎが発生した時点で epic 化を提案 (lazy promote safety net)
- 上記に該当しなければ default issue (type=task) で十分

## Lazy promote mechanics (γ rule) {#lazy-promote}

`bd update --type` flag は無いため、既存 issue ID を維持したまま parent-child で「暗黙 epic」を表現する:

```bash
bd create --parent=ori-XXX --title="残 sub-task A" -t task
bd create --parent=ori-XXX --title="残 sub-task B" -t task
```

`ori-XXX` 自体は `type=task` のまま。新 epic ID で wrap する β rule (ID 不連続) は user 明示要請時のみ。

**制約**: `bd epic status` / `bd epic close-eligible` は `type=epic` のみ filter するため、暗黙 epic はこれらに現れない。
epic 構造の確認は `bd dep tree ori-XXX` / `bd show ori-XXX`、close は `bd close ori-XXX` を手動で行う。

## Dispatch rule {#dispatch-rule}

User が ID のみ指定 (例: 「ori-XXX に取り組んで」) した時、`bd show` で状態判定して Mode を自動選択:

| 状態 | Mode | 振る舞い |
|---|---|---|
| has children (parent-child あり) | Mode-Epic | `bd ready` で child を依存解決順に取り組む。各 child は最小 ship 単位として Mode-Flat 再帰 |
| no children + in_progress + notes 履歴あり | Mode-Resume | notes 読込で進捗復元、TodoWrite reconstruct して続行 |
| no children + fresh / open | Mode-Flat | 直接実装。起動時 volume 判定で必要なら epic 化提案 |
| closed | — | "閉じています、新規 issue を作りますか?" と確認 |
| blocked | — | blocker 表示して停止 |

## Roadmap phase は label {#phase-as-label}

Phase / milestone は `bd create --labels=phase-x` で表現する。**epic として表現しない** (forever-open epic を避けるため)。

## Session 終了時の進捗保存 {#session-handoff}

- Mode-Flat / Mode-Resume で TodoWrite に残作業がある場合: 該当 bd issue の notes に進捗を append
  ```bash
  bd update ori-XXX --append-notes="session N: <次に何をするか / 残 TodoWrite items>"
  ```
- 次 session で Mode-Resume が notes から復元する
- 残作業が異質な複数 deliverable に分かれる場合は γ rule (lazy promote) で child 化

## `/ori-doctor` violation issue の label convention {#dod-violation-labels}

`/ori-doctor` が Slice DoD (`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の
"Slice Definition of Done") 違反を検出した時に起票する bd issue の label 規約。
violation 種別と所在を label から grep で復元できるようにする。

### 必須 label set

DoD violation issue は **以下 3 種を必ず付与** する:

| label | 形式 | 例 | 意味 |
| --- | --- | --- | --- |
| `dod-violation` | 固定文字列 | `dod-violation` | DoD 違反由来の issue であることを宣言。`/ori-doctor` 起票分すべてに付く marker |
| `slice:<slice-id>` | `slice:` + kebab-case slice ID | `slice:create-note` | 違反対象の slice。`manifest.yaml` の `id` と一致 |
| `rule:<rule-id>` | `rule:` + DoD rule ID | `rule:dod-2` | どの rule に違反したかを特定 (下表参照) |

### `rule:<rule-id>` の語彙

| rule ID | 違反内容 |
| --- | --- |
| `rule:dod-1` | `sub_layers` の全埋め違反 (`manifest.yaml` 宣言済み layer が空 or placeholder) |
| `rule:dod-2` | boundary 経由 test 違反 (tests が `application/` 等を直 import / bindings を経由していない) |
| `rule:dod-3` | production wiring 違反 (DoD test が `setupProductionBuilder()` 以外で fixture を組んでいる) |
| `rule:dod-4` | `cross_root` 同期切れ (generator 出力が source と不整合、例: `commands.rs` 更新後に bindings 再生成漏れ) |

### 補助 label (任意)

- `bc:<bc-name>`: 当該 slice が属する BC (`note-taking`, `task-management` 等)。
  複数 BC 横断の検索を高速化したい時に付与
- `phase:flow-impl-red` / `phase:flow-impl-green`: 違反が検出された phase を残す
  (rule:dod-4 で特に有用)

### Title / description 規約

- title: `[dod:<rule-id>] <slice-id>: <one-line summary>`
  例: `[dod:dod-2] create-note: tests が application/handle_create_note を直 import`
- description には **違反した file path と該当行** を必ず含める
  (例: `apps/notes/src/note_taking/slices/create-note/tests/create-note.test.ts:5`)
- 同一 slice / 同一 rule の重複起票を防ぐため、`/ori-doctor` は起票前に
  `bd list --label=dod-violation --label=slice:<id> --label=rule:<rule-id>`
  で既存 open issue を check する

### 起票コマンド例

```bash
bd create \
  --title="[dod:dod-2] create-note: tests が application を直 import" \
  --description="apps/notes/src/note_taking/slices/create-note/tests/create-note.test.ts:5 で handle_create_note を直 import。bindings 経由に書き換え必要" \
  --type=bug \
  --priority=2 \
  --labels=dod-violation,slice:create-note,rule:dod-2,bc:note-taking
```

## 公式 BOUNDARIES.md の core question {#core-question}

> **"Could I resume this work after 2 weeks away?"**
> - YES → bd で track
> - NO (markdown skim で十分) → TodoWrite で十分

迷ったらこの heuristic に従う。
