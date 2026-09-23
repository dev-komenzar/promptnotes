# Stack: TypeScript-Tauri — Test conventions

> pattern:ddd-vsa-hex × stack:typescript-tauri のテスト concretion 正典。
> 本 stack は **TS root (`vitest`) と Rust root (`cargo test`)** の 2 root を
> `cross_root: tauri-specta` で結ぶ。テスト規約もこの 2 root 分離を踏襲する。
>
> - stack-agnostic メタルール: `../pattern.md` "Test conventions"
> - TS 側 concretion の共通部分: `../typescript/test.md`
> - Rust 側 concretion の言語共通部分: `../rust/test.md`
> - 本ファイルは **Tauri 差分 (specta / bindings / mockIPC / production fixture)** のみを担う

## 概要

Tauri = 同一 app 内の TS/Rust 言語境界 (`cross_root`)。DoD の「boundary 経由 test」
(rule 2) は、Rust の `commands.rs`（`#[tauri::command]` 公開面）→ tauri-specta が
生成する TS 側 `shared/ipc/bindings.ts` → TS の DoD test が `commands.*` を invoke
という一方向の chain で満たす。テストの実体は TS 側 (vitest) に置き、Rust 側は
`cargo test` による内部 unit / proptest を補助とする。

## Slice DoD boundary test (`dod.test.ts`)

- **配置**: `<source_root>/<bc>/slices/<slice-id>/tests/dod.test.ts`
- **import 制約 (rule 2)**: `bindings` + `test-fixtures` + `vitest` +
  `@tauri-apps/api/mocks` のみ。`application/` / `infrastructure/` の直 import は
  DoD 違反（`/ori-doctor` が `dod-violation` label で起票）
- **production fixture (rule 3)**: `setupProductionBuilder()` を経由

```ts
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { commands } from '../../<bc>/shared/ipc/bindings';
import { clearProductionStore, setupProductionBuilder } from '../../<bc>/shared/test-fixtures';

describe('slice:<slice-id> DoD (boundary)', () => {
  beforeEach(() => mockIPC(setupProductionBuilder()));
  afterEach(() => { clearMocks(); clearProductionStore(); });
  it('spec.md#test-points: <観点> — succeeds via tauri-specta surface', async () => {
    const result = await commands.<sliceCamel>Cmd({ /* inputs per spec */ });
    expect(result).toMatchObject({ /* expected shape */ });
  });
});
```

## `setupProductionBuilder()` 規約 {#setup-production-builder}

- **配置**: `apps/<app>/src/<bc>/shared/test-fixtures/setupProductionBuilder.ts`
  （export name 固定）
- **戻り値**: tauri-specta `Builder` 相当。Rust 側 `commands.rs` の
  `#[tauri::command]` 公開関数群と一致する invoke handler を返す
- **slice 跨ぎ再利用**: 同 BC の全 slice DoD test が同一 `setupProductionBuilder()`
  を import。fake 用 `setup*Builder` を DoD test から import するのは禁止 (rule 3)

## Rust 側 boundary (commands.rs / specta / invoke_handler)

Rust 側の DoD 必須成果物の規約は本節 `#commands-rs-required` に一本化する。
言語共通の Rust 規約 (VO / Error / `unwrap()` 禁止) は `../rust/test.md` 参照。

## `commands.rs` 必須成果物 (Tauri stack) {#commands-rs-required}

`.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` の "Slice Definition of Done"
rule 2「boundary 経由 test」を Tauri stack で具体化する規約。

### 必須配置

```
apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/
├── mod.rs               # public entry (`pub use commands::*;` で command を再公開)
├── domain.rs
├── application.rs       # 副作用なし orchestration、内部からの直叩きは DoD 違反
├── infrastructure.rs
└── commands.rs          # ★ DoD 必須成果物: tauri-specta surface
```

- **`commands.rs` は slice DoD の必須成果物**: 欠けている slice は
  `manifest.yaml` の `expected_deliverables.boundary.kind: tauri_command` を
  満たしていないと判定される
- `commands.rs` 内の関数は `application::handle_*` を呼ぶ thin adapter に留め、
  domain ロジックを書かない

### Green 条件 (DoD rule 2 + rule 4)

slice が "green" と判定されるには **以下 3 点を同時に** 満たすこと:

1. **`#[tauri::command]` + `#[specta::specta]` で関数を export** していること

   ```rust
   // apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/commands.rs
   #[tauri::command]
   #[specta::specta]
   pub async fn create_note(
       state: tauri::State<'_, AppState>,
       raw_title: String,
   ) -> Result<NoteDto, AppError> {
       crate::<bc_rs>::slices::<slice_rs>::application::handle_create_note(
           &state, raw_title,
       )
       .await
   }
   ```

2. **`invoke_handler!` (or `tauri_specta::Builder::commands![...]`) に配線済み**
   であること。`lib.rs` の builder 構築箇所で当該 command が collect され、
   `.invoke_handler(builder.invoke_handler())` 経由で Tauri に登録される

   ```rust
   // apps/<app>/src-tauri/src/lib.rs
   let builder = tauri_specta::Builder::<tauri::Wry>::new().commands(
       tauri_specta::collect_commands![
           <bc_rs>::slices::<slice_rs>::commands::create_note,
           // ... other slice commands
       ],
   );
   ```

3. **tauri-specta generator (export-types bin) を走らせて TS 側 bindings が同期済み**
   であること。`/ori-flow` の `flow-impl-red-pre` / `flow-impl-green-post`
   phase hook で `cargo run --bin export-types` が呼ばれ、
   `apps/<app>/src/<bc>/shared/ipc/bindings.ts` を再生成する (DoD rule 4)

上記 3 点のいずれかが欠けると `/ori-doctor` は当該 slice を **DoD 違反** として
報告する (`task-management.instructions.md` の label convention 参照)。

### 内部直叩き禁止 (DoD rule 2)

- tests が `crate::<bc_rs>::slices::<slice_rs>::application::handle_*` を直 import
  するのは **DoD 違反**。tests は **必ず TS 側 bindings 経由** (`mockIPC` + 生成済み
  `commands` proxy) で呼ぶこと
- Rust crate 内に閉じた unit test (`#[cfg(test)] mod tests`) で fake を使った
  orchestration 検証を書くのは OK だが、DoD カウントには **含めない** (rule 3)

## 参照実装

- `example-slice/ts/task-management/slices/complete-task/tests/dod.test.ts`
- `example-slice/ts/task-management/shared/test-fixtures/setupProductionBuilder.ts`
- `example-slice/rust/task_management/slices/complete_task/commands.rs`
