---
name: ori-review
description: /ori-flow phase 6。fresh-context で ori-reviewer agent を起動し、impl を adversarial にレビューさせる薄いラッパー
---

ユーザが `/ori-review <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 6 として起動した際に、**`ori-reviewer` agent を fresh context で spawn し、その出力を捌く薄いラッパー**として動作します。**スキル本体はメイン session で動き、Bash 経由で reviewer agent を起動**します。

## 引数

- `slice-id`：対象 slice の id

## 役割

- **オーケストレーター**：reviewer agent を起動し、その指摘を集約
- **single-pass 強制装置**：往復は **最大 1 回**。無限ループに陥らないためのガード
- **patch ディスパッチャー**：指摘内容に応じて適切な phase（test-red / impl-green / refactor）に差し戻す

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/spec.md`
  - `.ori/slices/<id>/manifest.yaml`（`bc:` と `app:` の解決）
  - `.ori/config.yaml`（`workspace.apps:`、fallback `<source_root>` 解決）
  - `.ori/architecture.md`（あれば `root.path` / `roots[<id>].path` を canonical `<source_root>` として優先採用）
  - 実装 + テスト：`<source_root>/<bc>/slices/<slice-id>/{domain,application,infrastructure,presentation,tests}/`
  - BC 共有：`<source_root>/<bc>/{domain,shared/contracts/events}/`（slice が touch した範囲のみ）
  - 関連ドメイン文書：`manifest.derives_from`
- 出力：
  - `.ori/slices/<id>/review.md` — reviewer agent の指摘ログ
  - 必要なら beads issue の再 open（差し戻し先 phase）

## なぜ fresh context か

- 同一 session 内のレビューは認知バイアス（自分の実装を「正しい」と信じ込む）が強い
- ori-reviewer は **default capability: reasoning** で起動（current_agent 設定に従い claude-opus-4-7 / deepseek-v4-pro / o1 等）
- 実装者と異なる model を割り当てることで adversarial 視点を確保

## 手順

1. **前提確認**：
   - phase 5（refactor）完了が望ましいが、緊急時は phase 4 直後でも可
   - `pnpm -F <slice-pkg> test` が GREEN であることを確認
2. **`ori-reviewer` agent を fresh context で spawn**：
   - `ori-reviewer` の agent 指示を Read し、その全指示を Task agent のプロンプトに含める
   - 指定するファイルセット：`.ori/slices/<id>/{spec.md,manifest.yaml}`、`<source_root>/<bc>/slices/<slice-id>/{tests,domain,application,infrastructure,presentation}/`、manifest.derives_from の domain docs（`<source_root>` の解決は ori-impl-green と同じ。`.ori/architecture.md root.path` → `apps[].path + "/src"` の順）
   - reviewer 側に渡される入力：spec.md / tests / 関連 src / 関連 domain docs / 実装 diff
   - 総合判定（PASS / NEEDS_FIX / REJECT）を要求する
3. **reviewer の出力を受け取る**：
   - `.ori/slices/<id>/review.md` に書き込まれる
   - 形式：
     ```markdown
     ## Findings
     - **HIGH** test-perspectives#empty-body: spec が「破棄」と書いているが impl は error を返している
     - **MED**  impl-notes#throttle: throttle 値が hardcoded
     - **LOW**  test coverage: edge case missing for unicode whitespace
     ```
4. **指摘の処理**：
   - **指摘ゼロ** → phase 6 完了。`bd close ori-review-<slice-id>` で完了
   - **指摘あり**：severity と内容から差し戻し先を決定（**最大 1 回**）：
     | 指摘の性質 | 差し戻し先 |
     |----------|-----------|
     | spec の解釈ミス / 観点漏れ | `/ori-test-red`（観点追加） |
     | 実装の挙動が spec と乖離 | `/ori-impl-green`（修正） |
     | コード品質 / 重複 | `/ori-refactor` |
     | spec 自体が誤り | `/ori-propose`（domain 修正提案） |
5. **差し戻し後の再 review**：
   - patch 完了後、**1 回だけ** reviewer を再実行
   - **2 周目で再度指摘が出た場合は停止**し human flag：
     ```bash
     bd human ori-review-<slice-id> --reason="review loop reached 2nd pass; needs human arbitration"
     ```
6. **完了**：
   - `.ori/slices/<id>/review.md` を commit
   - `bd close ori-review-<slice-id> --reason="review passed; <N> findings addressed in <N> patches"`

## single-pass 強制

- 「reviewer → patch → reviewer」の往復は**最大 1 回**
- カウントは `review.md` の `## Pass 1` / `## Pass 2` 見出しで記録
- Pass 2 で新規指摘が出たら強制停止 → human

## 出力テンプレート

```markdown
# Review: capture-auto-save {#review-capture-auto-save}

## Pass 1 {#pass-1}

Reviewer: claude-opus-4-7 (capability=reasoning, fresh context)

### Findings

- **HIGH** spec.md#test-perspectives:
  - empty body の振る舞いについて spec は「破棄」と書いているが impl は `EmptyBody` error を返却している
  - 推奨: spec を明確化 + test を追加
- **LOW** apps/<app>/src/note-capture/slices/capture-auto-save/application/capture-auto-save.ts:
  - throttle 値が hardcoded (300ms)。spec で TBD のまま

### Disposition

- HIGH 指摘 → `/ori-test-red` に差し戻し（empty body の挙動を明示化）
- LOW 指摘 → `/ori-propose` で domain 側に throttle 規定追加を提案

## Pass 2 {#pass-2}

（必要時のみ）
```

## 注意

- **スキル本体はメイン session**：reviewer は Task agent で spawn する
- **single-pass 厳守**：3 周目に入ったら必ず human に上げる（無限ループ防止）
- **review.md は派生ファイルではない**：人間が読むための監査ログ。`coherence.source` は不要

## 次のアクション

phase 6 完了後、`/ori-flow` 内部なら自動的に phase 7 へ。単独呼び出しの場合：

- **メインパス**：`/ori-finalize <slice-id>` — phase 7。dirty 解除と必要に応じた proposal sync
- **差し戻しパス**：指摘の内容に応じて `/ori-test-red` / `/ori-impl-green` / `/ori-refactor` / `/ori-propose`
- **停止パス**：Pass 2 でも指摘が残った場合は human 判断待ち（`bd human` で flag 済み）
