---
name: ori-plan
description: /ori-flow phase 2。spec.md を読み、下流 phase の beads issue を idempotent に scaffold + description 展開する。plan.md ファイルは作らない（beads 単一情報源）
---

ユーザが `/ori-plan <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 2 として起動した際に、**spec.md の内容を下流 beads issue（test-red / impl-green / refactor / review / finalize）の description として展開**します。**plan.md は作らない**——タスク分解は beads が単一情報源。

下流 issue が未作成のときは **AI 自身が `bd create` で作成**します (idempotent、規約 ID `ori-<phase>-<slice-id>`)。CLI に `--setup-issues` 等の flag を追加する方針は廃止 (ori-execution-model-shift-2026-06-03, 旧 ori-100 close)。

## 引数

- `slice-id`：対象 slice の id（`.ori/slices/<id>/spec.md` が存在する前提）

## 役割

- **タスク展開者**：spec の `テスト観点` / `不変条件` / `実装ノート` を読み、下流 issue ごとに具体的な作業項目を割り当てる
- **beads scaffold + 編集者**：規約 ID に対して `bd create --id ...` (不在時) + `bd update <issue> --description=... --notes=...` で issue を埋める
- **境界守護者**：spec で TBD のままの項目は phase 2 で詰めるか、人間に投げ返す

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/spec.md`（phase 1 で生成）
  - 規約 ID の下流 beads issue (= 存在すれば既存利用、不在なら scaffold 対象):
    - `ori-test-red-<id>` / `ori-impl-green-<id>` / `ori-refactor-<id>` / `ori-review-<id>` / `ori-finalize-<id>`
  - 親 (parent): slice epic `ori-slice-<id>` (= `formatEpicId("slice", id)` 規約、`packages/slice-runner/src/beads.ts:7-11`)
- 出力（**ファイル無し / beads のみ更新**）：
  - 不在 issue に対し: `bd create --id ori-<phase>-<id> --parent ori-slice-<id> --type=task --priority=2 --title "..."`
  - 既存 / 新規いずれも: `bd update ori-<phase>-<id> --description=... --notes="checklist..."`
  - 各 phase は test-red / impl-green / refactor / review / finalize 全 5 件

## なぜ plan.md を作らないか

- **二重管理を避ける**：beads が task の SSoT。plan.md は drift する
- **進捗が見えにくくなる**：checklist は beads description の `- [ ]` で更新するため、ファイル化すると `git diff` ノイズになる
- **タスク粒度は phase 内に閉じる**：別 issue にしない（ori-flow.md 注意事項）

## 手順

1. **前提確認 + idempotent scaffold**：
   - `.ori/slices/<id>/spec.md` が存在し、TBD が解消されているか確認
   - **slice epic の存在確認** (Bash):
     ```bash
     bd search "ori-slice-<id>" --json
     ```
     結果に `"id": "ori-slice-<id>"` が無ければ作成:
     ```bash
     bd create --id "ori-slice-<id>" --type=epic --priority=2 \
       --title "slice: <id>" \
       --description "/ori-flow slice epic for <id>。子は phase 別 issue (ori-{derive,plan,test-red,impl-green,refactor,review,finalize}-<id>)。"
     ```
   - **plan 自身 + 下流 5 phase issue の idempotent scaffold** — 各 phase ∈ {`plan`, `test-red`, `impl-green`, `refactor`, `review`, `finalize`} について:
     ```bash
     bd search "ori-<phase>-<id>" --json
     ```
     結果に `"id": "ori-<phase>-<id>"` が無ければ作成:
     ```bash
     bd create --id "ori-<phase>-<id>" --parent "ori-slice-<id>" \
       --type=task --priority=2 \
       --title "phase=<phase>: <id>" \
       --description "/ori-flow phase <phase> for slice <id>. /ori-plan が中身を埋める。"
     ```
     存在すれば skip (description は次のステップ 3 で上書きされる)。
     - `ori-plan-<id>` を含める理由: 本 skill 自身が L5 「完了報告」で `bd close ori-plan-<id>` を呼ぶため、ここで存在保証する。
     - 単独呼び出し / `/ori-flow` 経由いずれでも skill が self-bootstrap できる。
   - **idempotency 警告** — 以下の **罠** がある:
     - `bd show <nonexistent>` は stderr に "no issue found" を出すが **exit code は 0**。`if bd show ...; then` では判定不能 → 必ず `bd search` の JSON を読む
     - `bd create --id <existing>` は**既存 issue を上書き**してしまう (title / description / status が初期化される) → 必ず `bd search` で先行確認すること
2. **spec.md を読み解く**：
   - `## テスト観点 {#test-perspectives}` → test-red の description
   - `## 不変条件 {#invariants}` → impl-green の checklist
   - `## 実装ノート {#impl-notes}` → impl-green / refactor の description
3. **各下流 issue を更新**（Bash）：
   - **test-red**：観点リストを `- [ ]` で description に埋める
     ```bash
     bd update ori-test-red-<id> --description="$(cat <<'EOF'
     spec.md#test-perspectives から導出した観点：

     - [ ] happy path: 通常入力 → 期待 event
     - [ ] empty body: 空白のみ → 破棄
     - [ ] non-existent: 不明 id → NoteNotFound
     - [ ] timestamp monotonic: updatedAt 増分検証
     EOF
     )"
     ```
   - **impl-green**：不変条件を保護する実装ステップを列挙
     ```bash
     bd update ori-impl-green-<id> --description="$(cat <<'EOF'
     - [ ] domain/vo/note-body.ts: smart constructor (whitespace reject)
     - [ ] domain/note.ts: editBody + updatedAt monotonic 保証
     - [ ] application/capture-auto-save.ts: pipeline composition
     - [ ] infrastructure/note-repository-memory.ts: in-memory impl for tests
     EOF
     )"
     ```
   - **refactor**：観点（重複除去・抽象化候補）を列挙。空でも良い
   - **review**：レビュー観点を列挙
     ```bash
     bd update ori-review-<id> --description="$(cat <<'EOF'
     - [ ] spec.md と impl の挙動乖離
     - [ ] 層配置（副作用が domain/ に漏れていないか）
     - [ ] テスト網羅性（unicode whitespace 等の edge case）
     - [ ] branded types の漏れ
     EOF
     )"
     ```
   - **finalize**：sync / proposal 必要性チェックを記載
4. **TBD の扱い**：spec に TBD が残っているなら：
   - 軽微（throttle 値など）→ phase 2 で人間に質問しその場で確定。spec を更新するなら `--force` 経路
   - 重大（不変条件不明など）→ 停止し `/ori-derive` への戻りを促す
5. **完了報告**：
   ```bash
   bd close ori-plan-<id> --reason="downstream issues populated: test-red/impl-green/refactor/review/finalize"
   ```

## 注意

- **plan.md ファイルは作らない**：beads description が SSoT
- **サブ issue を切らない**：description 内 `- [ ]` checklist で対応
- **TBD は積極的に詰める**：phase 2 の主目的の一つ
- **スキル自動化との関係**：`/ori-plan` は CLI を介さず、直接 spec.md を読み beads issue を更新する

## 次のアクション

phase 2 完了後、`/ori-flow` 内部なら自動的に phase 3 へ。単独呼び出しの場合：

- **メインパス**：`/ori-test-red <slice-id>` — phase 3。failing test を tests/ に書き起こす
- **TBD 残存パス**：`/ori-derive` で spec を再派生 or `/ori-propose` で domain 修正
