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
