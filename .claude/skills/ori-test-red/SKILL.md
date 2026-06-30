---
name: ori-test-red
description: /ori-flow phase 3。spec.md のテスト観点から failing test を <source_root>/<bc>/slices/<id>/tests/ に書き起こす。typescript-tauri stack では b3 (Slice DoD boundary test) として stub commands.rs → invoke_handler 登録 → specta rebuild → dod.test.ts emit → runtime RED (Err("pending")) を一気通貫で emit する
---

ユーザが `/ori-test-red <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 3 として起動した際に、**該当 slice の `<source_root>/<bc>/slices/<slice-id>/tests/` 配下に failing test を書く**。**impl (production application logic) は書かない**。RED が観測できた時点で完了。`<source_root>` は `.ori/architecture.md` の `root.path`（単一 root）または `roots[<id>].path`（multi-root）、なければ `.ori/config.yaml` `workspace.apps[<app>].path + "/src"` で resolve（後述）。

**stack=typescript-tauri** の場合は **Slice DoD rule 2/3** (`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の "Slice Definition of Done") を強制するため、test だけでなく **boundary 契約一式 (stub Rust command + invoke_handler 登録 + specta rebuild + bindings 経由 test)** を p3 sub-step として emit する (= **b3 emit**)。

## 引数

- `slice-id`：対象 slice の id（`.ori/slices/<id>/spec.md` が存在する事を前提）

## 役割

- **テスト設計者**：spec.md `## テスト観点` を vitest テストに展開
- **プロパティテスター**：value object の smart constructor は fast-check で生成テスト
- **トレーサビリティ守護者**：`it` の説明は spec.md の該当セクション id を引用する
- **境界契約 scaffolder** (typescript-tauri stack): stub Rust command を置き、invoke_handler 登録 + specta rebuild + dod.test.ts emit までを 1 phase で実施 (Slice DoD rule 2/3 の予約)
- **品質ゲート**：最初の実行で **すべて GREEN** になったら、それは spec / impl のどちらかにバグの兆候 → **強制停止**

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/spec.md`（phase 1 で生成済み。`## 境界契約 {#boundary-contract}` section を含む — `feature-spec.instructions.md` 参照）
  - `.ori/slices/<id>/manifest.yaml`（`bc:` `app:` と `expected_deliverables` の解決に必要）
  - `.ori/config.yaml`（`workspace.apps:` から `app:` 解決、fallback として `apps[].path`/src を `<source_root>` に使う）
  - `.ori/architecture.md`（あれば `root.path` / `roots[<id>].path` を canonical な `<source_root>` として優先採用。`stack:` field から typescript-tauri 判定）
  - `.apm/instructions/ddd-test.instructions.md`（domain test 規約）
  - `.apm/instructions/ddd-rust.instructions.md`（Rust stub 規約: commands.rs / invoke_handler! 配線）
  - `.apm/instructions/ui-test.instructions.md`（production fixture / `setupProductionBuilder()` 規約）
- 出力：
  - 全 stack: `<source_root>/<bc>/slices/<slice-id>/tests/<topic>.test.ts`
  - stack=typescript-tauri 追加:
    - `apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/commands.rs` (stub, `Err("pending")` 返却)
    - `apps/<app>/src-tauri/src/bin/export-types.rs` への `use` + `collect_commands![]` 追記
    - `apps/<app>/src-tauri/src/lib.rs` (or runtime builder entry) の `invoke_handler!` への登録
    - `<source_root>/<bc>/shared/ipc/bindings.ts` (specta-build で再生成、本 skill は直接書かない)
    - `<source_root>/<bc>/shared/test-fixtures/setupProductionBuilder.ts` への新 slice case 追記 (pending throw)
    - `<source_root>/<bc>/slices/<slice-id>/tests/dod.test.ts` (boundary 経由 invoke + production fixture)
  - test runner: vitest
  - property test: fast-check（VO の smart constructor）

## SSoT 参照表

| 規定 | 参照 |
| --- | --- |
| Slice DoD rules 1-4 | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` "Slice Definition of Done" |
| Test contract instantiation (typescript-tauri) | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/architecture.md.tpl` "Test Contract" section |
| 参照実装 (commands.rs / dod.test.ts / setupProductionBuilder.ts) | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/example-slice/` |
| 初期 scaffold (export-types.rs / specta-build.sh / setupProductionBuilder skeleton) | `.apm/skills/ori-init/scripts/install-tauri-scaffold.sh` + `.apm/skills/ori-init/scripts/templates/tauri-stack/` |

## `<app>` `<bc>` `<source_root>` `<stack>` の解決

skill 起動時に以下の順序で resolve:

1. **`<bc>`**：`.ori/slices/<id>/manifest.yaml` の `bc:` field
2. **`<app>`**：
   - manifest に `app:` field があれば優先採用
   - なければ `.ori/config.yaml` の `workspace.apps:` を参照
     - 要素 1 個 → その entry を採用
     - 要素 N 個 → エラー停止（manifest に `app:` を追加するよう user に促す）
   - config 未存在 → `/ori-init` 未実行エラー
3. **`<source_root>`**（test を書く base directory）：
   - **優先**: `.ori/architecture.md` が存在し `root.path`（単一 root）または `roots[<id>].path`（multi-root、manifest の `root:` field で選択）が設定されていればそれを採用
   - **fallback**: `.ori/architecture.md` 未生成なら `<workspace.apps[<app>].path>/src`（典型: `apps/<app>/src`）
   - **brownfield 例**: 既存 monorepo の `promptnotes/` subdir に `.ori/` を被せた場合、`workspace.apps[0].path: promptnotes` を設定すれば `<source_root>` は `promptnotes/src` に解決される
4. **`<stack>`**：`.ori/architecture.md` frontmatter の `stack:` field
   - `typescript-tauri` → b3 emit path (本 skill 主対象)
   - `typescript` / その他 → b3 emit は skip し、vitest test 単独 emit にフォールバック
5. **slice base**：`<source_root>/<bc>/slices/<slice-id>/`（出力先 path はこれを固定値として組み立てる）
6. **Rust 側 path** (stack=typescript-tauri のみ):
   - `<bc_rs>` = `<bc>` を snake_case 変換 (`task-management` → `task_management`)
   - `<slice_rs>` = `<slice-id>` を snake_case 変換 (`complete-task` → `complete_task`)
   - `<slice base rs>` = `apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/`
   - Rust 側 command 関数名 = `<slice_rs>_cmd` (例: `complete_task_cmd`)

## ddd-test.instructions 準拠ルール

| ルール | 内容 |
|--------|------|
| runner | vitest |
| 命名 | `describe('slice:<slice-id>', ...)` を必ず最外殻に |
| it 引用 | `it('spec.md#<section-id>: <観点>', ...)` 形式で spec へのリンクを残す |
| VO テスト | smart constructor は fast-check の `fc.property` で網羅 |
| import (domain test) | テスト対象は **sibling import**（`../application/handler.ts`, `../domain/vo/<name>.ts`） |
| import (DoD boundary test) | **bindings 経由のみ** (`<source_root>/<bc>/shared/ipc/bindings.ts` の `commands.*`)。`application/*` 直 import は **DoD rule 2 違反** |
| skip 禁止 | `it.skip` / `.todo` を使わない。書くなら失敗させる |

## 禁止事項

- **`.ori/slices/<id>/src/` への出力は禁止**。`.ori/slices/<id>/` は SSoT メタ専用（manifest.yaml / spec.md / status.yaml / notes.md 等のみ）。code および tests は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下に置く
- 配置先は skill 起動時に Bash で `mkdir -p <source_root>/<bc>/slices/<slice-id>/tests` を実行し、出力先を強制的に resolve 済み path に固定する
- **DoD boundary test (`dod.test.ts`) から `../application/*` / `../infrastructure/*` を直 import するのは禁止** (Slice DoD rule 2 違反 → `/ori-doctor` が `dod-violation` issue 起票)
- **fake / mock fixture を `dod.test.ts` で使うのは禁止** (rule 3 違反)。fake が必要な orchestration unit test は `application/` 内に co-locate

## 手順

1. **前提確認**：
   - `.ori/slices/<id>/spec.md` の `## テスト観点 {#test-points}` と `## 境界契約 {#boundary-contract}` を Read
   - 観点が空 / TBD のみなら停止し「先に `/ori-derive` で spec を埋めるか、ユーザに観点を確認」
   - manifest.yaml と `.ori/config.yaml` / `.ori/architecture.md` から `<app>` `<bc>` `<source_root>` `<stack>` を resolve
   - stack=typescript-tauri なら追加で `<bc_rs>` `<slice_rs>` を resolve
2. **テスト観点を列挙**：spec.md から bullet を抽出し、各観点を 1 つの `it` に対応付ける
3. **テストファイル / boundary scaffold の構成**：
   ```
   <source_root>/<bc>/slices/<slice-id>/tests/
     dod.test.ts                    ← b3 boundary test (stack=typescript-tauri 必須)
     <slice-id>.test.ts             ← happy path + 主要観点 (domain / application unit)
     <slice-id>-vo.property.test.ts ← VO smart constructor の fast-check（必要時）
     <slice-id>-edge.test.ts        ← edge case を集約（任意）
   ```
   stack=typescript-tauri の場合 (4) 〜 (8) を実行、それ以外は (9) へ skip
4. **stub Rust commands.rs を emit** (b3 step 1):
   - 配置: `apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/commands.rs`
   - 内容 (テンプレ、`<...>` を resolve 済 token で置換):
     ```rust
     // STUB: phase 3 (test-red). Real impl lands in phase 4 (impl-green).
     // Returns Err("pending") so /ori-impl-red can observe runtime RED via
     // the tauri-specta bindings (Slice DoD rule 2 boundary test).

     #[tauri::command]
     #[specta::specta]
     pub fn <slice_rs>_cmd(/* TODO inputs per spec.md#io */) -> Result<(), String> {
         Err("pending".to_string())
     }
     ```
   - 同階層に最小 `mod.rs` (なければ) と空 `domain.rs` / `application.rs` / `infrastructure.rs` placeholder も emit (DoD rule 1 sub_layer 全埋め確保)
   - 既存 stub があれば overwrite しない (再実行時の人間編集を保護)
5. **invoke_handler! 登録 patch** (b3 step 2):
   - `apps/<app>/src-tauri/src/lib.rs` の `tauri_specta::Builder` 構築箇所 (`.commands(collect_commands![...])`) に新 cmd を追記
   - `apps/<app>/src-tauri/src/bin/export-types.rs` にも同じく `use` + `collect_commands![]` 追記 (`install-tauri-scaffold.sh` が用意した entry に対する patch)
   - 既に登録済みなら no-op (idempotent)
6. **specta rebuild** (b3 step 3):
   - Bash で `bash apm-scripts/specta-build.sh --app-dir apps/<app>` を実行
   - 成功で `<source_root>/<bc>/shared/ipc/bindings.ts` が再生成され、`commands.<sliceRs>Cmd(...)` 型が増える
   - `cargo` 不在 (template script は exit 0 で warn) → スクリプト stderr を notes に転記し、bindings.ts を hand-edit せず人間判断を仰ぐ
   - `cargo` 不在以外の `cargo run` 失敗 → 停止 + `bd update --append-notes` で stderr 記録、人間判断
7. **setupProductionBuilder.ts に新 slice の pending case を追記** (b3 step 4):
   - 配置: `<source_root>/<bc>/shared/test-fixtures/setupProductionBuilder.ts`
   - 既存 `switch (cmd)` に新 case を append:
     ```ts
     case "<slice_rs>_cmd": {
       // STUB: phase 3 (test-red). Real production wiring lands in phase 4.
       throw new Error("pending");
     }
     ```
   - default fallthrough は維持 (未登録 slice は明示エラー)
   - 既に case がある (人間編集済 or 再実行) → overwrite しない
8. **DoD boundary test (`dod.test.ts`) を emit** (b3 step 5):
   - 配置: `<source_root>/<bc>/slices/<slice-id>/tests/dod.test.ts`
   - import は **bindings + test-fixtures + vitest + `@tauri-apps/api/mocks` のみ**
   - 本 skill の "出力テンプレート (DoD boundary)" を雛形として、spec.md の境界契約 / テスト観点を反映
   - 観点ごとに 1 `it`、`it` 名は `spec.md#<section-id>: <観点>` 形式
9. **domain / application 層の vitest テストを emit**:
   - 既存ルール (sibling import / fast-check VO test) で `<slice-id>.test.ts` / `<slice-id>-vo.property.test.ts` を書く
   - impl 不在の段階では module-not-found / type error で fail。`// @ts-expect-error` は不要、失敗をそのまま観測
10. **`pnpm test --filter <slice-id>` 相当を Bash で実行**して RED を確認：
    ```bash
    pnpm -F <app> test <source_root>/<bc>/slices/<slice-id>/tests
    ```
    - stack=typescript-tauri: `dod.test.ts` が `Error: pending` で fail することを期待 (b3 RED)
    - 全 stack: `<slice-id>.test.ts` / VO test は module-not-found / assertion fail で RED
11. **GREEN 観測時：強制停止**
    - 最初から全テストが GREEN なら spec か impl のどちらかにバグ可能性が高い
    - stack=typescript-tauri で `dod.test.ts` も最初から GREEN なら、stub commands.rs / setupProductionBuilder の throw が誤って削除されている疑い → diff 確認
    - bd issue にコメントを残して human flag：
      ```bash
      bd update ori-test-red-<slice-id> --notes="test was GREEN at first run — spec gap or stub overwritten"
      bd human ori-test-red-<slice-id> --reason="GREEN-on-first-run anomaly"
      ```
    - ユーザに「spec の観点が impl 済みの動作と合致しているか / stub が production wire に上書きされていないか確認してください」と促す
12. **RED 観測時：phase 3 完了**
    - 失敗テストの一覧 (どれが Err("pending") fail で、どれが module-not-found か) を beads notes に記録：
      ```bash
      bd update ori-test-red-<slice-id> --notes="b3 emit done: dod.test.ts RED via pending; N other RED via module-not-found"
      ```
    - `bd close ori-test-red-<slice-id>` で完了
13. lint / format：`pnpm lint --fix` を最後に走らせる (b3 で emit した dod.test.ts と setupProductionBuilder 追記分も対象)

## 失敗時のリカバリ

- テストファイル自体に文法エラー → **1 回だけ** 自動修正
- stub commands.rs / export-types.rs patch の文法エラー → **1 回だけ** 自動修正 (`cargo check` でローカル検証)
- specta-build.sh が非 cargo-missing 由来で fail → 停止して人間に判断を委ねる (deps 未追加 / lib.rs 未配線 / etc)
- それでも失敗 → 停止して人間に判断を委ねる

## 出力テンプレート

### DoD boundary (`dod.test.ts`, stack=typescript-tauri 必須)

```ts
// <source_root>/<bc>/slices/<slice-id>/tests/dod.test.ts
//
// Slice DoD boundary test (rule 2 + rule 3). Routes through the
// tauri-specta-generated bindings and the production fixture only —
// importing ../application/* or ../infrastructure/* from this file is
// a DoD violation (/ori-doctor will open a `dod-violation` issue).
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';

import { commands } from '../../../shared/ipc/bindings';
import {
  clearProductionStore,
  setupProductionBuilder,
} from '../../../shared/test-fixtures';

describe('slice:<slice-id> DoD (boundary)', () => {
  beforeEach(() => {
    mockIPC(setupProductionBuilder());
  });

  afterEach(() => {
    clearMocks();
    clearProductionStore();
  });

  it('spec.md#test-points: <観点1> — succeeds via tauri-specta surface', async () => {
    // expected to FAIL in phase 3 (stub returns Err("pending"))
    const result = await commands.<sliceCamel>Cmd({ /* inputs per spec */ });
    expect(result).toMatchObject({ /* expected shape per spec.md#io */ });
  });
});
```

### Stub Rust command (`commands.rs`, stack=typescript-tauri 必須)

```rust
// apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/commands.rs
//
// STUB emitted by /ori-test-red (phase 3). Real impl lands in phase 4
// (/ori-impl-green). Until then `Err("pending")` propagates through
// tauri-specta to the TS-side dod.test.ts as a runtime RED.

#[tauri::command]
#[specta::specta]
pub fn <slice_rs>_cmd(/* TODO inputs per spec.md#io */) -> Result<(), String> {
    Err("pending".to_string())
}
```

### Domain / application unit test (`<slice-id>.test.ts`)

```ts
// <source_root>/<bc>/slices/<slice-id>/tests/<slice-id>.test.ts
import { describe, it, expect } from 'vitest';
import { captureAutoSave } from '../application/capture-auto-save';

describe('slice:capture-auto-save', () => {
  it('spec.md#test-points: happy path → NoteSaved event', async () => {
    const result = await captureAutoSave({
      noteId: 'note-1',
      body: 'hello',
      occurredAt: new Date('2026-05-14T00:00:00Z'),
    });
    expect(result).toMatchObject({ type: 'NoteSaved' });
  });

  it('spec.md#test-points: empty body → not persisted', async () => {
    const result = await captureAutoSave({
      noteId: 'note-1',
      body: '   ',
      occurredAt: new Date('2026-05-14T00:00:00Z'),
    });
    expect(result).toMatchObject({ type: 'EmptyBody' });
  });
});
```

### VO property test (`<slice-id>-vo.property.test.ts`)

```ts
// <source_root>/<bc>/slices/<slice-id>/tests/<slice-id>-vo.property.test.ts
import { describe, it } from 'vitest';
import fc from 'fast-check';
import { NoteBody } from '../domain/vo/note-body';

describe('slice:capture-auto-save VO', () => {
  it('spec.md#invariants: NoteBody rejects whitespace-only', () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 1, maxLength: 50 }).filter((s) => s.trim() === ''),
        (input) => NoteBody.create(input)._tag === 'Left',
      ),
    );
  });
});
```

## 注意

- **impl (production application logic) を書かない**：stub commands.rs の `Err("pending")` 以外、`application/` `infrastructure/` には触らない。型シグネチャを想像で書きたくなっても禁止 (phase 4 の責務)
- **観点ごとに 1 it**：1 つの it に複数 expect を詰めない
- **GREEN-on-first-run は赤信号**：spec or impl にバグの兆候。安易にテスト追加で済まさない
- **`.ori/slices/<id>/` には絶対書かない**：code と tests は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下
- **dod.test.ts と domain unit test を混ぜない**：bindings 経由は DoD 用、sibling import は unit 用。1 ファイルに同居させると import 規約検査 (DoD rule 2) が複雑化する
- **stub Rust file 構造を保つ**：commands.rs 単独でなく `domain.rs` / `application.rs` / `infrastructure.rs` placeholder も emit して DoD rule 1 (sub_layer 全埋め) を p3 時点で予約する

## 次のアクション

phase 3 完了後、`/ori-flow` 内部なら自動的に phase 4 へ。単独呼び出しの場合：

- **メインパス**：`/ori-impl-green <slice-id>` — phase 4。stub commands.rs / `setupProductionBuilder` pending case を実装に差し替え、bindings 経由 test を GREEN にする
- **GREEN-on-first-run で停止した場合**：
  - 観点漏れなら spec を見直し `/ori-derive` で再派生
  - stub 上書き疑いなら `git log -p` で commands.rs / setupProductionBuilder の差分を確認
  - impl が予期せず存在するなら `/ori-doctor` で整合性検査
- **戻る**：spec の観点が貧弱なら `/ori-plan` で TBD を詰めるか domain に遡る
