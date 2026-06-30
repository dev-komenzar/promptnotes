---
name: ori-review
description: /ori-flow phase 6。3 つの structural gate (boundary test green / arch lint pass / public_entry 整合性) を機械実行し、すべて pass した時のみ fresh-context reviewer agent を spawn して spec 乖離のみ意味的に check する薄いゲート。Slice DoD は test contract で構造強制されるため独立 checklist を持たない
---

ユーザが `/ori-review <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 6 として起動した際に、**3 つの structural gate を Bash で実行 → すべて pass なら fresh-context で `ori-reviewer` agent を spawn → verdict と差し戻しを捌く薄いラッパー**として動作します。**スキル本体はメイン session で動き、Bash で gate を回し、reviewer は Task agent で起動**します。

## 引数

- `slice-id`：対象 slice の id

## 役割

- **3 gate ランナー**：boundary test / arch lint / public_entry の 3 check を Bash で実行
- **semantic reviewer ディスパッチャー**：3 gate pass 後に reviewer agent を fresh context で spawn (spec ↔ impl 乖離のみ意味的判定)
- **single-pass 強制装置**：往復は **最大 1 回**。無限ループに陥らないためのガード
- **patch ディスパッチャー**：指摘内容に応じて適切な phase（test-red / impl-green / refactor / propose）に差し戻す

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/spec.md`
  - `.ori/slices/<id>/manifest.yaml`（`bc:` `app:` と `expected_deliverables` 解決）
  - `.ori/config.yaml`（`workspace.apps:`、fallback `<source_root>` 解決）
  - `.ori/architecture.md`（あれば `root.path` / `roots[<id>].path` を canonical `<source_root>` として優先採用。`stack:` で gate (b) のコマンド分岐）
  - 実装 + テスト：`<source_root>/<bc>/slices/<slice-id>/{domain,application,infrastructure,presentation,tests}/`
  - BC 共有：`<source_root>/<bc>/{domain,shared/contracts/events,shared/ipc,shared/test-fixtures}/`（slice が touch した範囲のみ）
  - 関連ドメイン文書：`manifest.derives_from`
- 出力：
  - `.ori/slices/<id>/review.md` — 3 gate の log + reviewer agent の semantic 指摘
  - 必要なら beads issue の再 open（差し戻し先 phase）

## 3 structural gate (= ori-review が直接 check する全て)

| gate | check 内容 | 実行コマンド (典型例、stack 依存) |
| --- | --- | --- |
| **(a) boundary test green** | `<source_root>/<bc>/slices/<slice-id>/tests/` 配下 (`dod.test.ts` を含む) が GREEN | `pnpm -F <app> test <source_root>/<bc>/slices/<slice-id>/tests` |
| **(b) arch lint pass** | `/ori-arch` が生成した architecture adapter (eslint-plugin-boundaries / Rust `tests/arch.rs`) が pass | `pnpm -F <app> lint && (cd apps/<app>/src-tauri && cargo test --test arch)` (stack=typescript-tauri) / `pnpm -F <app> lint` (stack=typescript) |
| **(c) public_entry 整合性** | slice 外から slice 内部 (`domain/` `application/` `infrastructure/`) への直 import が無い (= `index.ts` / `mod.rs` 経由のみ)。大部分は (b) でカバーされるが spot grep で二重に確認 | `rg -n "slices/<slice-id>/(domain\|application\|infrastructure)/" <source_root> --glob='!**/slices/<slice-id>/**'` がヒット 0 件 |

3 gate のいずれかが fail なら **reviewer agent は spawn しない**。即 verdict を NEEDS_FIX or REJECT として該当 phase に差し戻す (詳細は手順 4)。

### なぜ DoD 個別 rules を review checklist にしないか {#why-no-dod-checklist}

Slice DoD (`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` "Slice Definition of Done") は **test contract で構造的に強制されている**ため、review が独立 checklist を持つと SSoT 二重化 → drift 源になる。各 DoD rule の検査責務は以下に分散済み:

| DoD rule | 強制責務 |
| --- | --- |
| rule 1 (sub_layers 全埋め) | `/ori-doctor` が `manifest.yaml#expected_deliverables.sub_layers` と実体ファイル/ディレクトリを突合 |
| rule 2 (boundary 経由 test) | `dod.test.ts` が `<bc>/shared/ipc/bindings` 経由でのみ command を呼ぶ — gate (a) が GREEN なら通っている。直 import は gate (b) の architecture adapter が reject |
| rule 3 (production wiring) | `dod.test.ts` から import 可能な fixture は `setupProductionBuilder` のみ — gate (b) の no-restricted-imports で機械的に enforce |
| rule 4 (cross_root 同期) | `flow-impl-red-pre` / `flow-impl-green-post` phase hook が `specta-build.sh` を走らせて bindings.ts を再生成。drift があれば gate (a) で型エラー fail |

そのため ori-review は **「3 gate + spec 乖離 semantic review」のみ** に絞る。`/ori-doctor` が DoD violation issue (`dod-violation` / `slice:<id>` / `rule:<rule-id>` label — `task-management.instructions.md` 参照) を起票する責務を引き受けるので、review が同じ check を繰り返さない。

## なぜ fresh context (reviewer agent) か

- 同一 session 内のレビューは認知バイアス（自分の実装を「正しい」と信じ込む）が強い
- ori-reviewer は **default capability: reasoning** で起動（current_agent 設定に従い claude-opus-4-7 / deepseek-v4-pro / o1 等）
- 実装者と異なる model を割り当てることで adversarial 視点を確保
- ただし reviewer の責務は **spec ↔ impl の意味的乖離のみ** — DoD / 層配置 / lint は 3 gate で機械処理済

## 手順

1. **前提確認**：
   - phase 5（refactor）完了が望ましいが、緊急時は phase 4 直後でも可
   - manifest.yaml / `.ori/config.yaml` / `.ori/architecture.md` から `<app>` `<bc>` `<source_root>` `<stack>` を resolve (`/ori-impl-green` と同じ手順)
2. **gate (a) boundary test green**：
   ```bash
   pnpm -F <app> test <source_root>/<bc>/slices/<slice-id>/tests
   ```
   - 1 つでも RED → `/ori-impl-green` に差し戻し (verdict=NEEDS_FIX、reason="boundary test RED")。手順 7 へ
3. **gate (b) arch lint pass**：
   ```bash
   pnpm -F <app> lint
   # stack=typescript-tauri なら追加で:
   ( cd apps/<app>/src-tauri && cargo test --test arch )
   ```
   - eslint / `arch.rs` のいずれかが fail → `/ori-refactor` に差し戻し (verdict=NEEDS_FIX、reason="arch lint violation"、詳細を review.md に貼る)。手順 7 へ
4. **gate (c) public_entry 整合性**：
   ```bash
   rg -n "slices/<slice-id>/(domain|application|infrastructure)/" "<source_root>" --glob='!**/slices/<slice-id>/**'
   ```
   - ヒットあり → `/ori-refactor` に差し戻し (verdict=NEEDS_FIX、reason="public_entry bypass")。手順 7 へ
   - ヒット 0 → gate 全 pass、reviewer spawn へ進む
5. **`ori-reviewer` agent を fresh context で spawn**：
   - `ori-reviewer` の agent 指示を Read し、その全指示を Task agent のプロンプトに含める
   - reviewer に渡す入力: `.ori/slices/<id>/{spec.md,manifest.yaml}`、`<source_root>/<bc>/slices/<slice-id>/{tests,domain,application,infrastructure,presentation}/`、manifest.derives_from の domain docs
   - reviewer に **明示**: **DoD 個別 check は不要** (3 gate で済んでいる)。**spec ↔ impl の意味的乖離 / 観点漏れ / 仕様解釈のずれ** のみ判定すること
   - 総合判定（PASS / NEEDS_FIX / REJECT）を要求する
6. **reviewer の出力を受け取る**：
   - `.ori/slices/<id>/review.md` に書き込まれる
   - 形式 (簡素):
     ```markdown
     ## Findings
     - **HIGH** test-points#empty-body: spec が「破棄」と書いているが impl は error を返している
     - **LOW**  edge case missing for unicode whitespace
     ```
7. **指摘の処理 (verdict logic — 維持)**：
   - **指摘ゼロ + 3 gate 全 pass** → verdict=PASS。`bd close ori-review-<slice-id>` で完了
   - **指摘あり**：severity と内容から差し戻し先を決定（**最大 1 回**）：
     | 指摘の性質 | 差し戻し先 | verdict |
     |----------|-----------|---------|
     | spec の解釈ミス / 観点漏れ | `/ori-test-red`（観点追加） | NEEDS_FIX |
     | 実装の挙動が spec と乖離 | `/ori-impl-green`（修正） | NEEDS_FIX |
     | コード品質 / 重複 / arch lint fail | `/ori-refactor` | NEEDS_FIX |
     | spec 自体が誤り | `/ori-propose`（domain 修正提案） | REJECT |
   - REJECT は人間判断必須 → `bd human` flag を立てて停止
8. **差し戻し後の再 review**：
   - patch 完了後、**1 回だけ** 手順 2 〜 6 を再実行
   - **2 周目で再度指摘が出た場合は停止**し human flag：
     ```bash
     bd human ori-review-<slice-id> --reason="review loop reached 2nd pass; needs human arbitration"
     ```
9. **完了**：
   - `.ori/slices/<id>/review.md` を commit
   - `bd close ori-review-<slice-id> --reason="3 gates pass + reviewer PASS; <N> findings addressed in <N> patches"`

## single-pass 強制

- 「gate → reviewer → patch → gate → reviewer」の往復は**最大 1 回**
- カウントは `review.md` の `## Pass 1` / `## Pass 2` 見出しで記録
- Pass 2 で新規指摘が出たら強制停止 → human

## 出力テンプレート

```markdown
# Review: capture-auto-save {#review-capture-auto-save}

## Pass 1 {#pass-1}

### Structural gates

- (a) boundary test: PASS (`pnpm -F notes test apps/notes/src/note-capture/slices/capture-auto-save/tests`)
- (b) arch lint:     PASS (`pnpm -F notes lint` + `cargo test --test arch`)
- (c) public_entry:  PASS (no external import bypassing index.ts/mod.rs)

### Semantic findings (reviewer: claude-opus-4-7, capability=reasoning, fresh context)

- **HIGH** spec.md#test-points:
  - empty body の振る舞いについて spec は「破棄」と書いているが impl は `EmptyBody` error を返却している
  - 推奨: spec を明確化 + test を追加
- **LOW** apps/notes/src/note-capture/slices/capture-auto-save/application/capture-auto-save.ts:
  - throttle 値が hardcoded (300ms)。spec で TBD のまま

### Disposition

- HIGH 指摘 → `/ori-test-red` に差し戻し (verdict=NEEDS_FIX、empty body の挙動を明示化)
- LOW 指摘 → `/ori-propose` で domain 側に throttle 規定追加を提案 (verdict=REJECT、human 判断待ち)

## Pass 2 {#pass-2}

（必要時のみ）
```

## 注意

- **独立 DoD checklist を持たない**: drift 源回避のため、DoD rule 1-4 を 1 個ずつ check しない。3 gate (test green / arch lint / public_entry) で構造的にカバーされる前提
- **3 gate fail 時は reviewer を spawn しない**: gate fail = 機械的な violation なので意味的 review にコストをかけない (gate を直して再実行)
- **reviewer の責務は spec ↔ impl 乖離のみ**: 層配置 / arch lint / DoD enforcement は spawn 前に既に終わっている
- **スキル本体はメイン session**：reviewer は Task agent で spawn する
- **single-pass 厳守**：3 周目に入ったら必ず human に上げる（無限ループ防止）
- **review.md は派生ファイルではない**：人間が読むための監査ログ。design.md §5 の `ori:` frontmatter は不要

## 次のアクション

phase 6 完了後、`/ori-flow` 内部なら自動的に phase 7 へ。単独呼び出しの場合：

- **メインパス**：`/ori-finalize <slice-id>` — phase 7。dirty 解除と必要に応じた proposal sync
- **差し戻しパス**：指摘の内容に応じて `/ori-test-red` / `/ori-impl-green` / `/ori-refactor` / `/ori-propose`
- **停止パス**：Pass 2 でも指摘が残った場合は human 判断待ち（`bd human` で flag 済み）
