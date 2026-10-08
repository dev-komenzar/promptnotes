---
name: ori-review
description: /ori-flow phase 6 (slice/page) または phase 3 (scenario)。slice/page の場合は 3 つの structural gate を機械実行し、すべて pass した時のみ fresh-context reviewer agent を spawn して spec 乖離のみ意味的に check する薄いゲート。scenario の場合は scenario spec とテストコードの整合性を review する。scenario では全 Then 句 ↔ assertion をカバレッジ gate し、未検証 Then は HIGH / NEEDS_FIX とする
---

ユーザが `/ori-review <id>` を呼ぶ、または `/ori-flow` 内部から phase 6 (slice/page) または phase 3 (scenario) として起動した際に、**slice/page の場合は 3 つの structural gate を Bash で実行 → すべて pass なら fresh-context で `ori-reviewer` agent を spawn → verdict と差し戻しを捌く薄いラッパー**として動作します。**scenario の場合は scenario spec とテストコードの整合性を review** します。**スキル本体はメイン session で動き、Bash で gate を回し、reviewer は Task agent で起動**します。

## 引数

- `id`：対象 slice / page / scenario の id

## 役割

- **structural gate ランナー**（slice/page のみ）：boundary test / arch lint / public_entry の 3 check (page / widget は + testid 契約 gate (d)) を Bash で実行
- **scenario reviewer**（scenario のみ）：scenario spec とテストコードの整合性を review し、全 `Then` 句 ↔ assertion のカバレッジを gate する
- **semantic reviewer ディスパッチャー**：structural gate pass 後に reviewer agent を fresh context で spawn (spec ↔ impl 乖離のみ意味的判定)
- **single-pass 強制装置**：往復は **最大 1 回**。無限ループに陥らないためのガード
- **patch ディスパッチャー**：指摘内容に応じて適切な phase（test-red / impl-green / refactor / propose / generate）に差し戻す

## 入力 / 出力

### slice/page の場合

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

### scenario の場合

- 入力：
  - `.ori/scenarios/<id>/spec.md`
  - `.ori/scenarios/<id>/manifest.yaml`
  - `.ori/scenarios/<id>/spec.md#scenario-steps`（Gherkin 形式のシナリオ。`Then` の出所）
  - `.ori/scenarios/<id>/tests/`（テストコード）
  - `.ori/scenarios/<id>/playwright.config.ts` / `wdio.conf.ts`（runner config。vitest は config なし）
  - `.ori/scenarios/<id>/docker-compose.yml`（compose-service 系参加時のみ。参加者ゼロなら不在も正常）
  - `.ori/architecture.md`（`workspace.apps[].runtime` / `scenario_test_runner` — 整合確認用）
- 出力：
  - `.ori/scenarios/<id>/review.md` — reviewer agent の semantic 指摘
  - 必要なら beads issue の再 open（差し戻し先 phase）

## 3 structural gate (= ori-review が直接 check する全て)

| gate | check 内容 | 実行コマンド (典型例、stack 依存) |
| --- | --- | --- |
| **(a) boundary test green** | `<source_root>/<bc>/slices/<slice-id>/tests/` 配下 (`dod.test.ts` を含む) が GREEN | `pnpm -F <app> test <source_root>/<bc>/slices/<slice-id>/tests` |
| **(b) arch lint pass** | `/ori-architect` が生成した architecture adapter (eslint-plugin-boundaries / Rust `tests/arch.rs`) が pass | `pnpm -F <app> lint && (cd apps/<app>/src-tauri && cargo test --test arch)` (stack=typescript-tauri) / `pnpm -F <app> lint` (stack=typescript) |
| **(c) public_entry 整合性** | slice 外から slice 内部 (`domain/` `application/` `infrastructure/`) への直 import が無い (= `index.ts` / `mod.rs` 経由のみ)。大部分は (b) でカバーされるが spot grep で二重に確認 | `rg -n "slices/<slice-id>/(domain\|application\|infrastructure)/" <source_root> --glob='!**/slices/<slice-id>/**'` がヒット 0 件 |
| **(d) testid 契約** (type: page / widget のみ。ori-oan.7) | `.ori/pages/<id>/testids.yaml` の契約 testid が実装に literal で存在し、derived が stale でなく、その page に帰属する実装 testid に動的組み立て・形式違反が無い (規範: `ddd-vsa-hex/pattern.md` "page / widget の testid 契約") | `node scripts/testids.js check <id>` が exit 0 |

gate のいずれかが fail なら **reviewer agent は spawn しない**。即 verdict を NEEDS_FIX or REJECT として該当 phase に差し戻す (詳細は手順 4)。

### なぜ DoD 個別 rules を review checklist にしないか {#why-no-dod-checklist}

Slice DoD ([`ori-architect/patterns/ddd-vsa-hex/pattern.md`](../../../apm_modules/dev-komenzar/ori/.apm/skills/ori-architect/patterns/ddd-vsa-hex/pattern.md) "Slice Definition of Done") は **test contract で構造的に強制されている**ため、review が独立 checklist を持つと SSoT 二重化 → drift 源になる。各 DoD rule の検査責務は以下に分散済み:

| DoD rule | 強制責務 |
| --- | --- |
| rule 1 (sub_layers 全埋め) | `/ori-doctor` が `manifest.yaml#expected_deliverables.sub_layers` と実体ファイル/ディレクトリを突合 |
| rule 2 (boundary 経由 test) | `dod.test.ts` が `<bc>/shared/ipc/bindings` 経由でのみ command を呼ぶ — gate (a) が GREEN なら通っている。直 import は gate (b) の architecture adapter が reject |
| rule 3 (production wiring) | `dod.test.ts` から import 可能な fixture は `setupProductionBuilder` のみ — gate (b) の no-restricted-imports で機械的に enforce |
| rule 4 (cross_root 同期) | `flow-impl-red-pre` / `flow-impl-green-post` phase hook が `specta-build.sh` を走らせて bindings.ts を再生成。drift があれば gate (a) で型エラー fail |

そのため ori-review は **「3 gate + spec 乖離 semantic review」のみ** に絞る。`/ori-doctor` が DoD violation issue (`dod-violation` / `slice:<id>` / `rule:<rule-id>` label — `task-management.instructions.md` 参照) を起票する責務を引き受けるので、review が同じ check を繰り返さない。

## なぜ fresh context (reviewer agent) か

- 同一 session 内のレビューは認知バイアス（自分の実装を「正しい」と信じ込む）が強い
- ori-reviewer は **default capability: reasoning** で起動（**scenario は例外**: 手順 5 の軽量既定 `deep` で spawn する。slice/page は reasoning）（current_agent 設定に従い claude-opus-4-7 / deepseek-v4-pro / o1 等）
- 実装者と異なる model を割り当てることで adversarial 視点を確保
- ただし reviewer の責務は **spec ↔ impl の意味的乖離のみ** — DoD / 層配置 / lint は 3 gate で機械処理済

## 手順

1. **前提確認と type 判定**：
   - manifest.yaml を読み、`type` フィールドを確認
   - `type: scenario` の場合 → §「scenario workflow」へ進む
   - `type: slice` または `type: page` の場合 → §「slice/page workflow」へ進む
   - `type` フィールドが存在しない場合 → エラーで停止し、ユーザに確認を求める

### slice/page workflow

2. **前提確認**：
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
   - ヒット 0 → slice は gate 全 pass、reviewer spawn へ進む。page / widget は gate (d) へ
4b. **gate (d) testid 契約** (manifest `type: page` / `type: widget` のみ):
   ```bash
   node scripts/testids.js check <id>
   ```
   - exit 1 → `/ori-impl-green` に差し戻し (verdict=NEEDS_FIX、reason="testid contract violation"、`VIOLATION` 行を review.md に貼る)。手順 7 へ
   - exit 0 → gate 全 pass、reviewer spawn へ進む
   - `NOTE impl:` 行 (どの page にも帰属できない実装違反) は gate の件数に含まれない。この page の変更で入ったものなら review.md の指摘に書く
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

### scenario workflow

2. **前提確認**：
   - phase 2（generate）完了が必須
   - manifest.yaml / spec.md / テストコードの存在を確認（`Then` の出所は `spec.md#scenario-steps`）
   - runner config を確認: spec.md の `runner:` 記録に従い `playwright.config.ts` / `wdio.conf.ts` が存在すること（vitest は config なしが正常）
   - docker-compose.yml: `infrastructure.services` に compose-service 系参加があるのに compose が不在、または逆（参加ゼロなのに存在）なら `/ori-generate` 差し戻し
3. **テストコードの構文チェック**：
   ```bash
   # TypeScript の場合
   npx tsc --noEmit .ori/scenarios/<id>/tests/*.spec.ts .ori/scenarios/<id>/*.config.ts
   ```
   - 構文エラー → `/ori-generate` に差し戻し (verdict=NEEDS_FIX、reason="test code syntax error")。手順 7 へ
4. **docker-compose.yml の構文チェック**（compose-service 系参加時のみ。不在なら skip）：
   ```bash
   docker compose -f .ori/scenarios/<id>/docker-compose.yml config -q
   ```
   - 構文エラー → `/ori-generate` に差し戻し (verdict=NEEDS_FIX、reason="docker-compose syntax error")。手順 7 へ
5. **`Then` 句の決定的列挙 → `ori-reviewer` agent を fresh context で spawn**：
   - **全 `Then` 句（Gherkin）を列挙**し、件数と一覧を reviewer に渡す（reviewer の網羅漏れ防止）。`spec.md#scenario-steps` の各ステップを列挙する:
     ```bash
     awk '/\{#scenario-steps\}/{f=1;next} /^## /{f=0} f && /^[[:space:]]*Then /{print FILENAME":"FNR": "$0}' .ori/scenarios/<id>/spec.md
     ```
   - 上の抽出結果が 0 件なら reviewer を spawn せず、verdict=NEEDS_FIX（reason="spec.md#scenario-steps に Then が無い"）で `/ori-derive` に差し戻す。手順 7 へ
   - `ori-reviewer` の agent 指示を Read し、その全指示を Task agent のプロンプトに含める
   - reviewer に渡す入力: `.ori/scenarios/<id>/{spec.md,manifest.yaml}`、`.ori/scenarios/<id>/test-points-map.md`、`.ori/scenarios/<id>/tests/`、runner config、`.ori/scenarios/<id>/docker-compose.yml`（あれば）、`.ori/architecture.md`、**上で列挙した `Then` 句の全件**
   - reviewer に **明示**: **各 `Then` をテストの assertion に対応付け、カバレッジ表（`Then` / 期待 assertion / テスト file:line / 状態）を出力**すること。加えて **scenario spec ↔ テストコードの整合性 / spec.md#scenario-steps の Gherkin シナリオ ↔ テストケースの対応**、**mode 別 checklist（下記）** を判定すること
   - reviewer に **明示**: **`Then` が 1 つでも UNVERIFIED（assertion 不在 / 最終状態しか見ない / 副作用・タイミング・フォーカス未検証）なら severity=HIGH + verdict=NEEDS_FIX。未検証 `Then` を LOW に disposition してはならない**。E2E で原理的に不能な項目は代替担保（unit test の file:line）を併記して `N/A(代替担保)` とし、代替が無ければ UNVERIFIED
   - **test-points 網羅**: `spec.md#test-points` の全項目を列挙して渡し（`scenario-test.instructions.md#test-points-map` の項目列挙 awk。`<spec>` = `.ori/scenarios/<id>/spec.md`）、`test-points-map.md` と突合させる。表の項目が spec と一致しない / `UNCOVERED` / 代替担保なしの `N/A(代替担保)` はいずれも HIGH / NEEDS_FIX（LOW 不可）。形式の SSoT は `scenario-test.instructions.md#test-points-map`
     ```bash
     awk '/^```/{c=!c} /^## .*\{#test-points\}/{f=1;next} /^## /{f=0} f && !c && /^- /' .ori/scenarios/<id>/spec.md
     ```
   - 総合判定（PASS / NEEDS_FIX / REJECT）を要求する

   **reviewer 実行の安定化（軽量既定 + timeout fallback）**:
   - **軽量既定**: scenario review の入力は小さい（spec + test + config）ため、reviewer は **軽量実行を既定**とする。reviewer には上記入力と `Then` 全件一覧だけを渡し、repo 全体の探索・追加調査は指示しない。capability は **`deep`** で spawn する（`reasoning` は要求しない）。根拠: `fast`(haiku) は adversarial 視点が弱く review に耐えず、`deep`(sonnet) は reasoning より軽く review 可能（`ori-model/SKILL.md` の capability 表）。Task agent spawn 時に model を `deep` の解決先へ**明示 override** する（agent frontmatter の `model:` / config の `ori-reviewer` override より優先）。reviewer が「深い推論が必要」と報告した場合のみ、main session が `reasoning` で **1 回だけ再 spawn** する（single-pass の往復カウントに含めない）
   - **timeout / 無応答 / 成果物なし**（harness の inactivity timeout で打ち切られた、または Task agent 終了後に手順 6 の `grep -m1 'Then ↔ assertion coverage' .ori/scenarios/<id>/review.md` が空＝成果物なし）は **review 未完了**であり、**PASS として扱ってはならない**。timeout を「指摘ゼロ」と解釈しない
   - **fallback 手順**（この 1 回のみ。single-pass の往復カウントには含めない）:
     1. reviewer を再 spawn せず、**main session が同じ入力で review を代行**する（`Then` 全件のカバレッジ表 / test-points 突合 / mode 別 checklist を自ら作成）
     2. `review.md` に `### Reviewer: main-session fallback (reason: reviewer timeout)` と明記して記録する（fresh context 保証が無いことを監査ログに残す）
     3. 以降は手順 6（機械検査）・手順 7（verdict logic）を通常どおり適用する。機械検査を通らなければ fallback 後も PASS にしない。手順 6 の「reviewer に再出力を要求」は **main session 自身による表の修正**と読み替える
     4. fallback でも完了できない場合は verdict を出さず `bd human ori-review-<scenario-id> --reason="reviewer timeout; fallback incomplete"` で停止する

   **mode 別 checklist（spec ↔ config ↔ compose 整合）**:
   - **共通**:
     - spec.md の `runner:` 記録 ↔ テストコードの import（playwright / wdio / vitest）が一致するか
     - テストコード内に service の起動・停止・healthcheck 待機が**無い**こと（lifecycle は runner config が所有 — `scenario-test.instructions.md`）
     - **`Then` ↔ assertion カバレッジ**: 列挙した全 `Then` がカバレッジ表で `VERIFIED` / `N/A(代替担保)` になっているか。`UNVERIFIED` が 1 つでもあれば HIGH / NEEDS_FIX（LOW 不可）
     - **副作用・タイミング・フォーカス・イベント**: 最終状態だけでなく、規定された副作用（focus 遷移 / debounce 時間 / event 発行）が assert されているか
     - **ドメイン規定数値**: domain 文書の数値・定数（debounce 等）がテストで assert され、実装定数と一致するか
   - **compose-service 系 app 参加時**:
     - manifest `infrastructure.services` ↔ docker-compose.yml の services が一致（過不足なし）か
     - runtime block の `ports` ↔ compose の ports ↔ runner config の待機 port が一致するか
     - app service に `environment` の接続 env が反映され、`TBD` マーカーが残っていないか（残っていれば人間判断待ち）
   - **local 系 app（tauri 等）参加時**:
     - local app が docker-compose.yml に**含まれていない**こと
     - wdio.conf.ts の `tauri:options.application` が runtime block の `binary` と一致するか（build-then-test）
     - `@wdio/tauri-service` が services に含まれるか
     - **前提条件（G1〜G6）の検証**: `spec.md` の実装ノートに前提条件（build 契約 / plugin / storage 隔離（XDG）/ symlink / fixture seed）が記録されているか
     - `runtime.build` が `runtime.binary` を生成する契約か（Tauri の `cargo build` 単体は devUrl 参照の dev binary なので不可 — G2）
     - `wdio.conf.ts` の `onPrepare` が次のいずれも所有しているか: 非 Linux / `:4444` 占有時の fail fast（`SevereServiceError` で throw。普通の `Error` は wdio に握りつぶされ続行する）/ node_modules symlink（G3）/ `XDG_CONFIG_HOME`・`XDG_DATA_HOME`・`XDG_CACHE_HOME`・`XDG_STATE_HOME` の per-run temp 設定と `ORI_SCENARIO_TMP` 公開、`runtime.test_env` の設定（G4）/ fixture seed（G6）。XDG 設定が test_env の後にあり上書きされないか。`maxInstances: 1` か。`TAURI_TEST_STORAGE_DIR` を test_env 宣言なしに注入していないか
     - テストコードが `browser.execute` の結果を `=== undefined` / `!== undefined` で判定していないか（WebDriver は `undefined` を `null` で返す。`!= null` が正。`scenario-test.instructions.md#browser-operations`）
     - plugin 警告の判定が `scenario-test.instructions.md#plugin-warnings` に従っているか（起動〜最初の reload まで 0、reload 直後の一過性警告は許容）
     - app 側に `tauri-plugin-wdio` の Rust 配線（Cargo dep / capabilities `wdio:default` / lib.rs `#[cfg(debug_assertions)]`）が施されているか（G1。未配線は `/ori-generate` へ差し戻し）
     - frontend の `@wdio/tauri-plugin` 動的 import と test build script が impl-notes の要求として記録されているか（G1）
   - **infra 参加時**:
     - infra の healthcheck（TCP probe の image ファミリ別翻訳）が compose に存在するか
     - `runtime.healthcheck: {http: /health}` 宣告のある app のみ HTTP 待機になっているか
6. **reviewer の出力を受け取る**：
   - reviewer が timeout / 無応答 / 成果物なしの場合は PASS にせず、手順 5 の fallback 手順に従う
   - `.ori/scenarios/<id>/review.md` に書き込まれる
   - 形式 (簡素):
     ```markdown
     ## Then ↔ assertion coverage
     | Then (source#anchor) | 期待 assertion | テスト (file:line) | 状態 |
     |---|---|---|---|
     | spec.md#scenario-steps Then 接続が成功する | connection 確立 | tests/x.spec.ts:42 | VERIFIED |
     | spec.md#scenario-steps Then フォーカスが移る | activeElement ∈ draft | — | **UNVERIFIED** |

     ## Findings
     - **HIGH** spec.md#scenario-steps: Then「フォーカスが移る」がテストで未検証
     - **LOW** docker-compose.yml: Redis の healthcheck が未設定
     ```
   - **カバレッジ表の実在を機械確認**（reviewer の出力漏れ防止・裁量排除）:
     ```bash
     test -n "$(grep -m1 'Then ↔ assertion coverage' .ori/scenarios/<id>/review.md)" \
       || echo "MISSING coverage table"
     # 状態セル（最終列）が UNVERIFIED の行数。凡例行を誤検知しないよう列末で一致させる
     grep -Ec '\|\s*(\*\*)?UNVERIFIED(\*\*)?\s*\|\s*$' .ori/scenarios/<id>/review.md   # 1 以上なら NEEDS_FIX
     ```
     - **test-points 対応表の機械検査**（reviewer の裁量排除）:
       ```bash
       m=.ori/scenarios/<id>/test-points-map.md
       test -f "$m" || echo "MISSING test-points-map.md"
       # spec の test-points 項目数 (SSoT: scenario-test.instructions.md#test-points-map の awk) と 表の行数 (TP-N 行) が一致すること
       awk '/^```/{c=!c} /^## .*\{#test-points\}/{f=1;next} /^## /{f=0} f && !c && /^- /' .ori/scenarios/<id>/spec.md | wc -l
       grep -Ec '^\|\s*TP-[0-9]+' "$m"
       # 状態列 (最終列) が UNCOVERED の行 / 代替担保列に file:line が無い N/A(代替担保) 行
       grep -Ec '\|\s*(\*\*)?UNCOVERED(\*\*)?\s*\|\s*$' "$m"
       awk -F'|' '/N\/A\(代替担保\)/ && $5 !~ /[^ ]+:[0-9]+/ {n++} END{print n+0}' "$m"
       ```
       - 表が無い / 項目数不一致 / `UNCOVERED` ≥ 1 / 代替担保列に file:line が無い `N/A(代替担保)` ≥ 1 → **reviewer の裁量に依らず** verdict=NEEDS_FIX、severity=HIGH、差し戻し先 `/ori-generate`
     - 表が無い / 列挙した `Then` 件数と表の行数が一致しない → review 不合格として reviewer に再出力を要求（この再出力は single-pass の往復カウントに含めない）
     - `UNVERIFIED` が 1 件以上 → **reviewer の裁量に依らず** verdict=NEEDS_FIX
7. **指摘の処理 (verdict logic — 維持)**：
   - **指摘ゼロ（カバレッジ表に UNVERIFIED ゼロ）** → verdict=PASS。`bd close ori-review-<scenario-id>` で完了
   - **指摘あり**：severity と内容から差し戻し先を決定（**最大 1 回**）：
      | 指摘の性質 | 差し戻し先 | verdict |
      |----------|-----------|---------|
      | テストコードの不備 | `/ori-generate`（テストコード再生成） | NEEDS_FIX |
      | runner config の不備 | `/ori-generate`（runner config 再生成） | NEEDS_FIX |
     | docker-compose の不備 | `/ori-generate`（docker-compose 再生成） | NEEDS_FIX |
     | 前提条件（plugin / build / env）の欠落 | `/ori-generate`（app 前提 patch 再実行） | NEEDS_FIX |
     | test-points 未カバー / 代替担保の記載漏れ | `/ori-generate`（テスト追加 or 代替担保明記） | NEEDS_FIX |
     | `Then` 未検証（assertion 不在） | `/ori-generate`（assertion 追加） | NEEDS_FIX |
     | spec 自体が誤り | `/ori-propose`（domain 修正提案） | REJECT |
   - **`Then` 未検証は severity 下限 HIGH**（LOW への disposition 不可）。カバレッジ表に UNVERIFIED が 1 つでもあれば verdict は NEEDS_FIX
   - REJECT は人間判断必須 → `bd human` flag を立てて停止
8. **差し戻し後の再 review**：
   - patch 完了後、**1 回だけ** 手順 2 〜 6 を再実行
   - **2 周目で再度指摘が出た場合は停止**し human flag：
     ```bash
     bd human ori-review-<scenario-id> --reason="review loop reached 2nd pass; needs human arbitration"
     ```
9. **完了**：
   - `.ori/scenarios/<id>/review.md` を commit
   - `bd close ori-review-<scenario-id> --reason="reviewer PASS; <N> findings addressed in <N> patches"`
   - **phase 台帳の更新（R1）**: `node scripts/scenario-status.js set <scenario-id> review done`（決定的 writer。冪等）

## single-pass 強制

- 「gate → reviewer → patch → gate → reviewer」の往復は**最大 1 回**
- カウントは `review.md` の `## Pass 1` / `## Pass 2` 見出しで記録
- Pass 2 で新規指摘が出たら強制停止 → human

## 出力テンプレート

### slice/page の場合

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

### scenario の場合

```markdown
# Review: test-scenario {#review-test-scenario}

## Pass 1 {#pass-1}

### Syntax checks

- test code: PASS (`npx tsc --noEmit .ori/scenarios/test-scenario/tests/*.test.ts`)
- docker-compose: PASS (`docker-compose -f .ori/scenarios/test-scenario/docker-compose.yml config`)

### Then ↔ assertion coverage (reviewer: fresh context)

| Then (source#anchor) | 期待 assertion | テスト (file:line) | 状態 |
|---|---|---|---|
| spec.md#scenario-steps Then 接続が成功する | connection 確立 | tests/test-scenario.test.ts:42 | VERIFIED |
| spec.md#scenario-steps Then キーが設定される | store に key が存在 | tests/test-scenario.test.ts:88 | VERIFIED |
| spec.md#scenario-steps Then フォーカスが移る | activeElement ∈ draft | — | **UNVERIFIED** |

### Semantic findings (reviewer: claude-sonnet-4-6, capability=deep, fresh context)

- **HIGH** spec.md#scenario-steps:
  - Then「フォーカスが移る」がテストで assert されていない（カバレッジ表 UNVERIFIED）
  - 推奨: テストコードに focus 遷移の assertion を追加
- **LOW** docker-compose.yml:
  - Redis の healthcheck が未設定
  - 推奨: healthcheck を追加してサービス起動を待つ

### Disposition

- HIGH 指摘 → `/ori-generate` に差し戻し (verdict=NEEDS_FIX、テストコード再生成)
- LOW 指摘 → `/ori-generate` で docker-compose 修正 (verdict=NEEDS_FIX)

## Pass 2 {#pass-2}

（必要時のみ）
```

## 注意

- **独立 DoD checklist を持たない** (slice/page): drift 源回避のため、DoD rule 1-4 を 1 個ずつ check しない。3 gate (test green / arch lint / public_entry) で構造的にカバーされる前提
- **3 gate fail 時は reviewer を spawn しない** (slice/page): gate fail = 機械的な violation なので意味的 review にコストをかけない (gate を直して再実行)
- **reviewer の責務は spec ↔ impl 乖離のみ** (slice/page): 層配置 / arch lint / DoD enforcement は spawn 前に既に終わっている
- **reviewer の責務は spec ↔ テストコード整合性のみ** (scenario): docker-compose の構文チェックは spawn 前に既に終わっている
- **`Then` 未検証は PASS を妨げる** (scenario): カバレッジ表に `UNVERIFIED` が 1 つでもあれば verdict は NEEDS_FIX、severity は HIGH 以上。LOW への disposition は不可
- **reviewer timeout は PASS ではない** (scenario): 無応答・成果物なしは review 未完了。main session fallback（手順 5）で review を完遂し、機械検査を通すまで PASS にしない。fallback も不能なら human に上げる
- **E2E 不能項目は代替担保で担保** (scenario): `N/A(代替担保)` は unit test の file:line を併記した場合のみ有効。代替が無ければ `UNVERIFIED`
- **スキル本体はメイン session**：reviewer は Task agent で spawn する
- **single-pass 厳守**：3 周目に入ったら必ず human に上げる（無限ループ防止）
- **review.md は派生ファイルではない**：人間が読むための監査ログ。design.md §5 の `ori:` frontmatter は不要

## 次のアクション

phase 6 (slice/page) または phase 3 (scenario) 完了後、`/ori-flow` 内部なら自動的に次 phase へ。単独呼び出しの場合：

- **slice/page のメインパス**：`/ori-finalize <slice-id>` — phase 7。dirty 解除と必要に応じた proposal sync
- **scenario のメインパス**：`/ori-finalize <scenario-id>` — phase 4。dirty 解除と spec hash 更新
- **差し戻しパス**：指摘の内容に応じて `/ori-test-red` / `/ori-impl-green` / `/ori-refactor` / `/ori-propose` / `/ori-generate`
- **停止パス**：Pass 2 でも指摘が残った場合は human 判断待ち（`bd human` で flag 済み）
