---
paths:
  - "src/**/*.rs"
---

- **VO**: newtype pattern + `try_new` で Smart Constructor。`pub struct NoteId(String);` + `impl NoteId { pub fn try_new(raw: &str) -> Result<Self, NoteIdError> { ... } }`
- **`#[derive(Debug, Clone, PartialEq, Eq, Hash)]`** を VO に標準で付ける
- **Error は `thiserror`**: `#[derive(Error, Debug)]`
- **Tauri command**: 入力は raw types で受け取り、内部で `try_new_*` を呼ぶ
- **Aggregate state は `&self -> (Self, Vec<Event>)` 形式の純粋関数**
- **fs / clock など副作用は trait で抽象化**：mock 可能に
- **テスト**: `cargo test` + `proptest` (property test)
- **`unwrap()` 禁止**: `?` または `expect("invariant: ...")` で理由を明示

## Slice 完了の必須成果物: `commands.rs` (Tauri stack) {#commands-rs-required}

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
