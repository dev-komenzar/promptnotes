# Stack: Rust — 言語共通テスト規約

> Rust 言語自体の concretion 正典（Tauri 有無を問わない）。
> Tauri stack (`typescript-tauri`) の Rust 側は本ファイルに Tauri 差分を上乗せする。
> 参照: `../typescript-tauri/test.md`（Tauri 固有 specta/commands surface）。

## VO / Smart Constructor

- **newtype pattern + `try_new` で Smart Constructor**:
  ```rust
  pub struct NoteId(String);
  impl NoteId {
      pub fn try_new(raw: &str) -> Result<Self, NoteIdError> { /* ... */ }
  }
  ```
- **`#[derive(Debug, Clone, PartialEq, Eq, Hash)]`** を VO に標準で付ける

## Error

- **`thiserror`** を使用: `#[derive(Error, Debug)]`

## 副作用の抽象化

- fs / clock などの副作用は **trait で抽象化**（mock 可能に）
- Aggregate state は **`&self -> (Self, Vec<Event>)` 形式の純粋関数**

## `unwrap()` 禁止

- `?` または `expect("invariant: ...")` で理由を明示

## テスト

- **ランナー**: `cargo test`
- **property test**: `proptest`
- Rust crate 内の unit test (`#[cfg(test)] mod tests`) で fake を使った
  orchestration 検証は OK だが、DoD カウントには含めない（rule 3）

## Tauri command surface（typescript-tauri stack のみ）{#tauri-command-surface}

Tauri を併用する stack にのみ該当する concretion。

- **入力は raw types で受け取り、内部で `try_new_*` を呼ぶ**:
  ```rust
  #[tauri::command]
  #[specta::specta]
  pub async fn create_note(
      state: tauri::State<'_, AppState>,
      raw_title: String,
  ) -> Result<NoteDto, AppError> { /* application::handle_* へ委譲 */ }
  ```
- `commands.rs` の関数は `application::handle_*` を呼ぶ thin adapter に留め、
  domain ロジックを書かない
