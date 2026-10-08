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
- frontmatter `ori:` ブロックがあるか（`node_id` / `type` / `depends_on`、design.md §5）
- 必須セクション（slice / page / phase ごと）の有無

### 2. 派生文書の hash 一致

- `.ori/slices/*/status.yaml` の `upstream_hash` と現在の domain section ハッシュを比較
- 不一致なら **dirty 残存** として報告

### 3. Dirty integrity (unauthorized clear detection)

- `.ori/slices/*/status.yaml` で `dirty: []` なのに `review.md` が存在しない / verdict が PASS でない slice を検出
- `clear-dirty.sh` をバイパスして `sed` 直打ち等で dirty が消された可能性を示唆

### 4. status.yaml ⇔ beads の同期

- 各 slice について `status.yaml.phase_status` と `bd show ori-<phase>-<id>` の `status` を突き合わせる
- ズレを報告（例：beads では closed だが status.yaml では in_progress）

### 5. Cross-reference の整合

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

### 8. Slice DoD sweep {#dod-sweep}

各 slice について Slice DoD ([`ori-architect/patterns/ddd-vsa-hex/pattern.md`](../../../apm_modules/dev-komenzar/ori/.apm/skills/ori-architect/patterns/ddd-vsa-hex/pattern.md) の "Slice Definition of Done") 4 rule を sweep する。

### 9. architecture.md guardrails 検証 (ori-c79.3)

`.ori/architecture.md` が存在する場合、`ori-architect` SKILL.md の構造セクション
(`invariants` / `guardrails`) を YAML fenced block から machine parse し、guardrails
g-1..g-8 を適用して適合判定する (lint.js 内で自動実行)。

- `g-1` — frontmatter が `parseArchitectureSpec()` を pass する (version=1、root/roots 必須)
- `g-2` — 使用 layer_set が `invariants.layer_graph` と一致 (layers / cross_layer / same_layer / public_entry_required)
- `g-3` — 使用 slice_internal が `invariants.slice_internal` と一致 (sub_layers / rules)
- `g-4` — `cross_slice`: prohibited_direct=true、via に shared/contracts + shared/events を含む
- `g-5` — `cross_bc`: BC 間は app-level shared/ 経由、same_event_bus=true
- `g-6` — 全 root で public_entry_required=true、public_entry が 1 ファイル
- `g-7` — cross_root は generator 明示 + auto_generated=true (生成物は手書き禁止)
- `g-8` — decision_points (platforms / os_integration / ui_native) の回答が `## Decisions` 節
  か frontmatter `decisions:` に記録されている

ori-architect SKILL.md ([`ori-architect/SKILL.md`](../../../apm_modules/dev-komenzar/ori/.apm/skills/ori-architect/SKILL.md)) が見つからない場合は検証をスキップ
(旧 doctor 挙動を維持)。報告のみで自動修正はしない。

- `rule:dod-1` — `manifest.yaml` の `expected_deliverables.sub_layers` で宣言した layer が `<source_root>/<bc>/slices/<slice-id>/<layer>/` (TS) or `apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/<layer>.rs` (Rust) に実体を持つか
- `rule:dod-2` — Tauri stack の slice について tests が `<bc>/shared/ipc/bindings` 経由で invoke しているか、かつ `application/` を直 import していないか
- `rule:dod-3` — tests が `setupProductionBuilder` を経由しているか (heuristic、参照無し → 違反疑い)。`--run-tests` 指定時は production fixture 経由で test を実 invoke し fail を rule:dod-3 違反として記録
- `rule:dod-4` — `commands.rs` の mtime が `bindings.ts` より新しい場合は specta 再生成漏れとして起票 (再生成 path は `apm-scripts/specta-build.sh --app-dir apps/<app>` — `architecture.md` の `phase_hooks.flow-impl-green-post` 由来)

read-only mode (default) は report のみ。`--dod-sweep` (= 内部 script `--emit-issues`) 指定時のみ bd issue を自動起票する。issue の label / title / description 規約は [`task-management.instructions.md`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/task-management.instructions.md) の "`/ori-doctor` violation issue の label convention" を SSoT として参照 (このスキル内に hardcoded 化しない)。

**Idempotency**: 起票前に `bd list --label=dod-violation --label=slice:<id> --label=rule:<rule-id> --status=open` を check し、既存 open issue があれば re-file しない (slice + rule の組で dedupe)。

### 10. scenario coverage (validation.md ↔ .ori/scenarios) {#scenario-coverage}

`.ori/domain/validation.md` の scenario section のうち `.ori/scenarios/<id>/manifest.yaml` が無いもの（= 未 scaffold）を **WARN** として surface する。

- coverage は `validation.md` と `.ori/scenarios/` から**毎回 live 導出**する。新規 registry ファイル（`coverage.yaml` 等）は作らない（drift 源になる）
- 導出は `node scripts/new-scenario.js --list-validation` の出力（`candidate (not scaffolded ...)` 行 / `coverage: N/M scaffolded`）を再利用する
- 各 candidate に fix 案内 `node <ori-flow>/scripts/new-scenario.js <id>`（scaffold 後 `/ori-flow <id>`）を付ける
- 全 section scaffolded なら clean（WARN なし）
- **ERROR にしない**：skip（意図的に scenario 化しない）を記録する機構が無く、false positive が恒久化するため。**skip 記録は保留**：WARN が常時出て無視される状態になったら skip 記録の導入を再検討する
- `/ori-distill` Phase 7 では scaffold 判断をしない（arch 前では早すぎる）。検出は doctor で行う

### 11. scenario 台帳整合 (status.yaml ⟷ 成果物) {#scenario-schema}

`.ori/scenarios/<id>/status.yaml` の `phases` / `beads.completion` と成果物の実在を突合し、台帳 drift を **WARN** で surface する（`check-scenario-schema.sh`）。

- 検出: `status.yaml` 不在 / `tests/` あり・`review.md` ありなのに該当 phase が done でない（`phases: {}` + 成果物 = s1/s4 型）/ phase=done なのに成果物なし / `completion` と `phases` の不整合 / legacy scalar 形式の phase / review=done なのに `finalize` 未記録（s2 型）
- 判定根拠に `spec.md` は使わない：`new-scenario.js` が scaffold 時点で作るため derive 済みの証拠にならない
- `finalize` は固有の成果物が無いので「review=done なのに finalize 未記録」（review=done は ori-review が PASS 時にのみ記録するので、`review.md` の verdict 書式には依存しない）で代替検出する。進行中の scenario でも当たるため WARN 文言に「進行中なら無視可」を明記する
- **ERROR にしない**：進行中 scenario で doctor を fail させないため
- fix は `node <ori-flow>/scripts/scenario-status.js set <id> <phase> <state>` を案内する
- status.yaml は `yaml.stringify` の block style 前提（flow style 手編集は未対応）

### 12. scenario review 陳腐化 (review.md ⟷ spec/tests mtime) {#scenario-staleness}

`review.md` の mtime が同 scenario の入力成果物（scenario 直下の `spec.md` / `manifest.yaml` / `validation.md` / `playwright.config.ts` / `wdio.conf.ts` / `teardown.mjs` / `tsconfig.json` / `docker-compose.yml`、および `tests/**`。`status.yaml` と `review.md` 自身は除く）より古い scenario を **WARN** で surface する（`check-scenario-staleness.sh`）。spec/tests を再生成・編集したのに旧 review.md が残っている状態（review の invalidation 機構が無い gap）の検出。

- fix は `/ori-flow <id>` の再走（review→finalize の再実行）を案内する
- 判定は mtime のみ。同秒は stale にしない。git checkout/clone は mtime を揃えるため検出漏れ（false negative）になりうる
- `review.md` が無い scenario は対象外（未 review は §11 が扱う）
- **ERROR 表記にしない**：再生成直後など進行中の scenario で ERROR 扱いにしないため（WARN でも `check-*.sh` は件数を exit code で返すので `run-checks.sh` の集計には数えられる）

### 13. ui-fields contract traceability (ui-fields ⟷ spec#test-points) {#ui-fields-traceability}

`.ori/domain/ui-fields/` の field contract が、いずれかの `spec.md#test-points` に写っていないものを **WARN** で surface する（`check-ui-fields-traceability.sh`）。Cmd+N のような cross-screen shortcut が未実装・未検証のまま GREEN になる再発防止（R9、将来枠の最小実装）。

- 対象 contract: (a) `screen-*.md` の field id `{#screen-N-xxx}`、(b) `ui-fields/*.md` 全体（cross-screen shortcut を定義する index / README を含む）のショートカット（`Cmd/Ctrl/⌘ + Key`、`Shift`/`Alt` 併用含む）
- 写像判定は単語境界つきの文字列一致（大小無視）。field は field id 自体、または purpose 部分（`screen-N-` prefix を除いた部分。`-` と空白は同一視）が出れば写像済み（`titlebar` は `title` に一致しない）。ショートカットは Cmd/Command/Ctrl/Control/⌘ の表記ゆれと `+` 前後の空白、`⌘S` の `+` 省略を同一視し、key は複数文字（`Enter` 等）を右境界つきで比較する（`Cmd+Enter` は `Cmd+E` で写像済みにならない）
- corpus は `.ori/scenarios/*/spec.md` / `.ori/pages/*/spec.md` / `.ori/slices/*/spec.md` の `#test-points`（sub-bullet 含む、fenced code 除外）を連結したもの。どの spec に写すべきかは判定しない
- `ui-fields/*.md` が無い、または test-points が 1 つも無ければ skip
- fix は該当 spec の `#test-points` への項目追加 → `/ori-flow <id>` 再生成を案内する
- **ERROR にしない**：文字列一致のため false positive がありうる。ori-arch adapter（page spec への構造的な写像）との連携は未実装（R4/R5 安定後の将来課題）

### 14. page testid 契約 (testids.yaml ⟷ ui-fields ⟷ 実装) {#page-testids}

`.ori/pages/<id>/testids.yaml` (page / widget の testid 契約。規範は `ddd-vsa-hex/pattern.md` "page / widget の testid 契約") を全 page 横断で検査する（`check-page-testids.sh` = 同 bundle の `scripts/testids.js check --all`、ori-oan.7）。

- 検出: testids.yaml 不在 / derived が ui-fields と不一致 (stale) / extra 形式違反・重複 / **契約 testid が実装 source に literal で不在** (契約 ⊆ 実装) / 実装 testid の動的組み立て・形式違反・存在しない page 参照
- 実装 testid の違反は page に帰属させる (`page.<id>` / `widget.<id>` はその page、`screen-<N>-*` はその screen を持つ page、それでも決まらなければ同じファイルの testid から 1 page に決まればその page)。決まらないものは `impl` (起票時は `page:_impl`)
- scenario-first で `/ori-generate` が後から `extra:` に追記した行 (`source: scenario:<id>`) の実装追従漏れもここで拾う
- 実装探索は `.ori/config.yaml` `workspace.apps[].path` 配下 (node_modules / target / dist / テストファイルを除外)
- fix: 既存実装の移行は `/ori-bug` から [`ui-test.instructions.md#testid-migration`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/ui-test.instructions.md#testid-migration) の手順へ (置換対応表は `node scripts/testids.js migrate-map <page-id>`)、stale は `node scripts/testids.js sync <page-id>`
- `check-page-testids.sh` は `packages/skills-shared/src` から ori-doctor / ori-generate に複製配置される。`/ori-generate` は実装のある参加 page だけを `--implemented-only <page-id>...` で検査・起票する
- `--testid-sweep` 指定時は page ごとに bd issue を起票 (手順 3b)

## 手順

1. **`.ori/` 存在確認**：なければ「`/ori-init` で初期化してください」と返す
2. **スクリプトで全検査を実行**：
   ```bash
   bash ./scripts/run-checks.sh
   ```
   個別検査は以下で構成：
   - `check-domain-schema.sh` — ドメイン文書の frontmatter + anchor 検証
   - `check-slice-schema.sh` — slice の manifest/status ファイル存在確認（status.yaml 不在 slice を WARN + 2 段の fix（status.yaml 復元 → `/ori-flow <id>`）付きで報告）
   - `check-scenario-schema.sh` — scenario の status.yaml 台帳 ⟷ 成果物実在の突合（phases 空/未記録なのに tests/・review.md が存在、phase=done なのに成果物なし、completion と phases の不整合を WARN。fix は `scenario-status.js set` を案内）
   - `check-scenario-staleness.sh` — review.md が spec.md / runner config / tests/** などの入力成果物より古い scenario を WARN（mtime 比較、fix は `/ori-flow <id>` 再走）
   - `check-ui-fields-traceability.sh` — ui-fields の field id / ショートカットが scenario・page・slice の spec.md#test-points に未写像なら WARN（文字列一致、fix は test-points 追記 → `/ori-flow <id>`）
   - `check-dirty-integrity.sh` — dirty=[] なのに review.md 不在/verdict≠PASS を検出（status.yaml の手動改竄チェック）
   - `check-hash-consistency.sh` — 派生ファイルの upstream 参照実在確認
   - `check-cross-ref.sh` — derives_from / upstream の cross-reference 検証
   - `check-proposals.sh` — pending proposal カウント
   - `check-scenario-coverage.sh` — validation.md の未 scaffold scenario section を WARN（live 計算、`new-scenario.js --list-validation` 再利用）
   - `check-dod-sweep.sh` — Slice DoD 4 rule の sweep (read-only mode、report のみ)
   - `check-page-testids.sh` — page / widget の testid 契約 (testids.yaml) の stale・形式・実装追従・実装 testid lint を WARN（§14）
   - `lint.js` — `.ori/` の Markdown anchor / id 規約検証 + architecture.md guardrails 検証（JS）：
     ```bash
     node ./scripts/lint.js [<path>] [--strict]
     ```
     `<path>/architecture.md` が存在する場合は `ori-architect` SKILL.md の guardrails
     g-1..g-8 適合判定も同時に実行する
3. **`/ori-doctor --dod-sweep` 指定時**: 上記に加えて DoD sweep を **issue auto-emit mode** で再実行：
   ```bash
   bash ./scripts/check-dod-sweep.sh --emit-issues
   # CI/full-check mode (heavy):
   bash ./scripts/check-dod-sweep.sh --emit-issues --run-tests
   ```
   - 違反ごとに bd issue を起票 (label 規約 SSoT は `task-management.instructions.md`)
   - idempotent: `bd list --label=dod-violation --label=slice:<id> --label=rule:<rule-id> --status=open` が hit するなら re-file しない
3b. **`/ori-doctor --testid-sweep` 指定時**: testid 契約違反を **issue auto-emit mode** で再実行：
   ```bash
   bash ./scripts/check-page-testids.sh --emit-issues
   ```
   - page ごとに 1 issue (label `testid-violation` + `page:<id>`、page に帰属できない実装 testid lint は `page:_impl`。label 規約 SSoT は `task-management.instructions.md`)。issue 本文に移行手順 ([`ui-test.instructions.md#testid-migration`](../../../apm_modules/dev-komenzar/ori/.apm/instructions/ui-test.instructions.md#testid-migration)) を書く
   - idempotent: `bd list --label=testid-violation --label=page:<id> --status=open,in_progress,blocked` が hit するなら re-file しない
4. **結果を集約**してレポートを生成
5. **報告 only**：自動修復は行わない (DoD sweep の auto-emit は例外として bd issue を作るが、code は触らない)

## レポートフォーマット

```
🩺 ori-doctor report

═══ Domain Schema ═══
✓ .ori/domain/discovery.md
✗ .ori/domain/aggregates.md:42 — H2 "Note Aggregate" missing {#id}
  fix: edit aggregates.md, add anchor manually (human judgment)

═══ Slice Schema (status.yaml 不在 slice) ═══
⚠ slices/detect-external-changes: no status.yaml (created outside new-slice.js or legacy); /ori-flow cannot finalize until restored
  fix: 1) restore .ori/slices/detect-external-changes/status.yaml, 2) /ori-flow detect-external-changes

═══ Hash Consistency ═══
⚠ slices/capture-auto-save: 1 upstream out of sync
  upstream: domain/aggregates.md#note-aggregate
  fix: /ori-flow capture-auto-save (re-derive)

═══ Dirty Integrity ═══
✗ slices/delete-note: dirty=[] but review.md not found
  fix: /ori-flow delete-note (run full workflow including review)

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

═══ Scenario Coverage ═══
⚠ validation.md#s22-storage-dir-change-watcher-restart — scenario 未 scaffold
  fix: node <ori-flow>/scripts/new-scenario.js s22-storage-dir-change-watcher-restart → /ori-flow s22-storage-dir-change-watcher-restart
  scenario coverage: 21/22 scaffolded, 1 candidate(s)

═══ Scenario Schema ═══
⚠ scenarios/s1: tests/ が存在するが phases.generate=(未記録) (台帳 drift)
  fix: node <ori-flow>/scripts/scenario-status.js set s1 generate done

═══ Beads ═══
✓ bd doctor: all green

═══ DoD Sweep ═══
✗ [rule:dod-2] create-note — tests が application/ を直 import
    detail: apps/notes/src/note_taking/slices/create-note/tests/create-note.test.ts:5:import handle_create_note from "../application/...
    ✓ filed bd issue (dod-violation, slice:create-note, rule:dod-2)
✗ [rule:dod-4] archive-task — commands.rs が bindings.ts より新しい (specta 再生成漏れ)
    detail: bash apm-scripts/specta-build.sh --app-dir apps/notes を実行して同期し、再 sweep してください
    INFO: existing open issue found — skipping re-file (idempotent)

=== DoD sweep summary: 2 violation(s) across 3 slice(s) ===

═══ Architecture Guardrails ═══
✗ [g-3] slice_internal "slice-internal-ts" (layer_set "ddd-vsa-hex-ts" の domain) が architecture.md に未宣言
✗ [g-8] decision_points の回答記録がない (## Decisions 節 か frontmatter decisions: が必要)
  fix: /ori-architect で再生成させる (要件対話から)

═══ Summary ═══
✗ 4 errors  ⚠ 2 warnings  ℹ 2 info
recommended action: fix broken cross-ref first (blocks /ori-flow on edit-past-note-start)
```

> 注: Slice Schema の status.yaml 不在は WARN のみ（ERROR 化しない）。進行中 slice も含むため `/ori-feature-status` の表示（phase=scaffold / last activity=(not started)）と突き合わせて判断する。

### status.yaml 復元手順（Slice Schema WARN の fix 1 段目）

`new-slice.js` は既存ディレクトリがあると `Slice already exists` で終了するため再実行では復元できない。`/ori-finalize` の `clear-dirty.sh` も status.yaml 不在だと `not found` で停止する。したがって `/ori-flow <id>` の前に `.ori/slices/<id>/status.yaml` を手で作る（`new-slice.js` の初期形）:

```yaml
slice_id: <id>
derived_at: <ISO8601 timestamp>
beads:
  epic: <slice epic id>   # new-slice.js は formatEpicId("slice", id) で生成。既存の bd epic があればその id
  current_phase: null
  completion: []
phases: {}
dirty: []
```

復元用スクリプトは提供しない（検出/修復機構の新設は別 issue）。

## 注意

- **自動修復しない**：domain 文書の手入れ・spec の再 derive はそれぞれ別スキル
- **read-only**：このスキル自体は何もファイルを変更しない（副作用ゼロ）
- **CI 統合可能**：将来 `/ori-doctor --json` 相当の出力をパイプして CI gate に使う想定

## 次のアクション

レポート内容に応じて以下を案内：

- **schema 違反パス**：`vim .ori/domain/<file>.md` で手動修正（自動修正しない）→ 再度 `/ori-doctor`
- **hash 不一致パス**：`/ori-flow <id>` で該当 slice / page を再 derive
- **dirty integrity 違反パス**：`/ori-flow <id>` で該当 slice の全 phase（特に review）を再実行。status.yaml を手動編集せず、`/ori-finalize` の `clear-dirty.sh` を経由すること
- **broken cross-ref パス**：該当 slice / page の `manifest.yaml` を更新 or 旧 anchor を domain 側で復活
- **proposal 残存パス**：`/ori-review-proposals` で人間判断
- **orphan domain パス**：意図的なら無視、不要なら削除を検討
- **beads 不整合パス**：`bd dolt push` / `bd dolt pull` で再同期、`bd orphans` で個別対処
- **DoD 違反パス**: 該当 slice の missing artifact を `/ori-impl-red` (b3 stub) / `/ori-impl-green` (real impl + production wiring + specta post) で生成。`rule:dod-4` は `bash apm-scripts/specta-build.sh --app-dir apps/<app>` で再同期
- **architecture.md guardrails 違反パス**: `/ori-architect` で要件対話から再生成させる (self-check → confirm で再検証)。`g-8` だけの場合は `## Decisions` 節か frontmatter `decisions:` への回答記録を追加 (自動修正しない)
- **scenario 未 scaffold パス**: `node scripts/new-scenario.js <id>` で scaffold → `/ori-flow <id>`。意図的に scenario 化しない section は現状 skip 記録の機構が無い（WARN のまま。常時 WARN で無視される状態になったら skip 記録を再検討）
- **全部 green パス**：`/ori-feature-status` で次の作業候補を選ぶ
