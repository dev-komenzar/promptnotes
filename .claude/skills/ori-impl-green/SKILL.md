---
name: ori-impl-green
description: /ori-flow phase 4。failing test を GREEN にする最小実装を <source_root>/<bc>/slices/<id>/ 配下に書く（DDD-VSA-Hex レイアウト準拠。<source_root> は `.ori/architecture.md` root.path または `apps[].path`/src で resolve）
---

ユーザが `/ori-impl-green <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 4 として起動した際に、**phase 3 で書いた failing test を GREEN にする最小実装を `<source_root>/<bc>/slices/<slice-id>/` 配下に書く**。**過剰な抽象化は phase 5（refactor）の責務**。`<source_root>` は `.ori/architecture.md` の `root.path`（単一 root の場合）または `roots[<id>].path`（multi-root）、なければ `.ori/config.yaml` `workspace.apps[<app>].path + "/src"` で resolve します（後述）。

## 引数

- `slice-id`：対象 slice の id（`tests/` に failing test が存在する事を前提）

## 役割

- **最小実装者**：テスト 1 本ずつ通す。投機的な拡張は書かない
- **DDD レイヤー守護者**：副作用は `infrastructure/` 層にのみ置く。`domain/` と `application/` は pure
- **進捗トラッカー**：beads issue description の `- [ ]` checklist を完了ごとに `- [x]` へ更新（**サブ issue を切らない**）

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/spec.md`
  - `.ori/slices/<id>/manifest.yaml`（`bc:` と `app:` の解決、`expected_deliverables` (DoD) の取得に必要）
  - `.ori/config.yaml`（`workspace.apps:` から `app:` 解決、fallback として `apps[].path`/src を `<source_root>` に使う）
  - `.ori/architecture.md`（`root.path` / `roots[].path` を canonical な `<source_root>` として優先採用、`cross_root` の有無で Tauri stack 判定、`phase_hooks.flow-impl-green-post` を読み specta 再生成 step を実行）
  - `<source_root>/<bc>/slices/<slice-id>/tests/*.test.ts`（phase 3 で RED 確認済み）
  - 実装規約 (SSoT):
    - `.apm/instructions/ddd-typescript.instructions.md`
    - `.apm/instructions/ddd-rust.instructions.md` (Tauri stack の場合、特に "commands.rs (Tauri stack)" section)
    - `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` (Slice DoD)
    - `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/example-slice/` (worked code)
- 出力：
  - `<source_root>/<bc>/slices/<slice-id>/{domain,application,infrastructure,presentation}/...`
  - Tauri stack: `apps/<app>/src-tauri/src/<bc_rs>/slices/<slice_rs>/{domain.rs,application.rs,infrastructure.rs,commands.rs}` (commands.rs は stub `Err("pending")` → real impl 置換)
  - Tauri stack: `apps/<app>/src/<bc>/shared/test-fixtures/setupProductionBuilder.ts` (新 command の dispatch arm を追加)
  - Tauri stack: `apps/<app>/src/<bc>/shared/ipc/bindings.ts` (specta 生成、手書き禁止)
  - `<source_root>/<bc>/slices/<slice-id>/tests/` 配下の全テストが GREEN (TS boundary test + Rust cargo test)

## `<app>` `<bc>` `<source_root>` の解決

skill 起動時に以下の順序で resolve:

1. **`<bc>`**：`.ori/slices/<id>/manifest.yaml` の `bc:` field
2. **`<app>`**：
   - manifest に `app:` field があれば優先採用
   - なければ `.ori/config.yaml` の `workspace.apps:` を参照
     - 要素 1 個 → その entry を採用
     - 要素 N 個 → エラー停止（manifest に `app:` を追加するよう user に促す）
   - config 未存在 → `/ori-init` 未実行エラー
3. **`<source_root>`**（code/test を書く base directory）：
   - **優先**: `.ori/architecture.md` が存在し `root.path`（単一 root）または `roots[<id>].path`（multi-root、manifest の `root:` field で選択）が設定されていればそれを採用
   - **fallback**: `.ori/architecture.md` 未生成なら `<workspace.apps[<app>].path>/src`（典型: `apps/<app>/src`）
   - **brownfield 例**: 既存 monorepo の `promptnotes/` subdir に `.ori/` を被せた場合、`workspace.apps[0].path: promptnotes` を設定すれば `<source_root>` は `promptnotes/src` に解決される
4. **slice base**：`<source_root>/<bc>/slices/<slice-id>/`（出力先 path はこれを固定値として組み立てる）

## 禁止事項

- **`.ori/slices/<id>/src/` への出力は絶対に禁止**。`.ori/slices/<id>/` は SSoT メタ専用（manifest.yaml / spec.md / status.yaml / notes.md / plan.md / review.md のみ）。code は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下に書く
- skill 起動時に出力先 path を resolve 済み変数に固定し、相対 path で書く際も resolve 済み base から組み立てる
- 出力直前に `pwd` 相当を確認、`.ori/slices/<id>/src/` が存在したら停止 + bd issue にエラー記録

## ddd-typescript.instructions 準拠ルール

| ルール | 内容 |
|--------|------|
| ディレクトリ | `<source_root>/<bc>/slices/<slice-id>/{domain,application,infrastructure,presentation,tests}/`（典型: `apps/<app>/src/...`） |
| BC 共有 | aggregate / event 等の BC 共有型は `<source_root>/<bc>/{domain,shared/contracts/events}/`（Phase 10 types 生成領域、slice からは import） |
| Branded types | `type NoteId = string & { readonly __brand: 'NoteId' }` 形式 |
| Smart constructor | VO は `class.create(raw): Result<VO, Error>` 形式。直接 new を export しない |
| Result type | エラーは throw せず `Result<T, E>`（または `Either`）で返す |
| 副作用配置 | I/O は `infrastructure/`。`domain/` と `application/` は pure |
| 依存方向 | `infrastructure → application → domain`（逆向き禁止） |
| import | 集約をまたぐ参照は repository interface 経由のみ |

## 手順

1. **前提確認**：
   - manifest.yaml と `.ori/config.yaml` / `.ori/architecture.md` から `<app>` `<bc>` `<source_root>` を resolve
   - 出力 base を `<source_root>/<bc>/slices/<slice-id>/` に固定
   - **stack 判定**: `.ori/architecture.md` frontmatter の `cross_root` が non-empty なら **Tauri stack mode** (Rust 側 commands.rs を含む)、無ければ **pure TS stack mode**
   - `pnpm -F <app> test <base>/tests` を Bash で実行し RED であることを確認（phase 3 完了の検証）
   - Tauri stack mode: `cargo test --manifest-path apps/<app>/src-tauri/Cargo.toml` も実行し Rust 側 RED 確認
   - 既に GREEN なら停止し phase 3 へ差し戻す（`/ori-test-red` の "GREEN-on-first-run" と同等）

2. **TS 側のテストを 1 本ずつ通す**（pure TS stack のみ; Tauri stack では step 3 を優先）：
   - 一番外側の `it` から順に attack
   - 「テストを通すための最小限のコード」だけ書く（YAGNI）
   - 関連する VO / entity / workflow ステップを `<base>/domain/` に追加
   - I/O が必要なら `<base>/infrastructure/` に repository 実装を追加し、`<base>/application/` で DI

3. **Tauri stack の場合: Rust 側を最小実装で GREEN にする** (DoD rule 2/3/4 → `pattern.md` + `ddd-rust.instructions.md` の "commands.rs (Tauri stack)"):
   1. **stub `commands.rs` の `Err("pending")` を real impl で置換**: thin adapter として `application::handle_<verb>_<noun>` を呼ぶだけ。domain logic は書かない (`ddd-rust.instructions.md` の Green 条件 #1)
   2. **`application.rs` / `infrastructure.rs` の production adapter 実装**: domain 関数を組み合わせ、infrastructure adapter (DB / repository / clock 等) を DI で受け取る。Repository trait は domain で宣言、実装は infrastructure (`ddd-rust.instructions.md` の依存方向)
   3. **`lib.rs` の `tauri_specta::Builder::commands![...]` に当該 command を追加配線** (Green 条件 #2)
   4. **Cargo deps の追加** (必要な場合): scaffold 由来の `tauri-specta` / `specta` / `specta-typescript` は ori-fzr.5 の `install-tauri-scaffold.sh` で配置済み。新規に必要な crate (例: `serde` / `chrono` / `uuid`) は `cd apps/<app>/src-tauri && cargo add <crate>` で追加
   5. **package.json deps の追加** (TS 側 fixture 用): `@tauri-apps/api/mocks` を含む `@tauri-apps/api` は upstream `pnpm tauri init` で配置済み。新規 fixture 依存 (例: `vitest`) が未配置なら `pnpm -F <app> add -D <pkg>`
   6. **`shared/test-fixtures/setupProductionBuilder.ts` に dispatch arm を追加**: 新 command の `case "<cmd_name>"` を追加し、production wiring (実 adapter set + Rust handler invoke) を仕込む。fake/mock を返さない (DoD rule 3)。雛形は `install-tauri-scaffold.sh` 配置のもの、書き方は `ddd-rust.instructions.md` を参照

4. **層配置のチェック**:
   - `domain/` に I/O 依存がないか (TS / Rust 共通)
   - `commands.rs` 内に domain logic が漏れていないか (`application::handle_*` を呼ぶ thin adapter に留めるか)
   - tests が **`application::handle_*` を直 import していないこと** を確認 (DoD rule 2 違反)
   - 集約をまたぐ呼び出しが repository interface 経由か
   - branded types / VO が裸の primitive で漏れていないか

5. **specta rebuild post-step** (Tauri stack のみ; `phase_hooks.flow-impl-green-post` で declarative 化されている = `architecture.md` 由来):
   ```bash
   bash apm-scripts/specta-build.sh --app-dir apps/<app>
   ```
   - bindings.ts が再生成され、新 command の TS signature が反映される (DoD rule 4)
   - phase_hooks 経由なので、`/ori-flow` が自動 invoke する場合は手動実行不要 — 但し skill 単独呼び出し時は明示実行すること

6. **進捗の記録**: beads issue description の checklist を Bash で更新:
   ```bash
   bd update ori-impl-green-<slice-id> --notes="step N done: <topic>"
   ```
   - **サブ issue は切らない**（ori-flow.md 注意事項）

7. **全テスト GREEN を確認**:
   ```bash
   # TS 側 (boundary test 含む)
   pnpm -F <app> test <base>/tests
   pnpm -F <app> typecheck

   # Tauri stack のみ — Rust 側 cargo test
   cargo test --manifest-path apps/<app>/src-tauri/Cargo.toml
   ```
   - 失敗時は step 8 のリカバリへ

8. **lint / format**:
   ```bash
   pnpm lint --fix
   pnpm format
   # Tauri stack のみ
   cargo fmt --manifest-path apps/<app>/src-tauri/Cargo.toml
   cargo clippy --manifest-path apps/<app>/src-tauri/Cargo.toml -- -D warnings
   ```

9. **出力先の self-check**:
   ```bash
   # 禁止 path への漏出が無いことを確認
   test ! -d .ori/slices/<slice-id>/src || (echo "ERROR: .ori/slices/<id>/src must not exist" && exit 1)
   # Tauri stack: bindings.ts が specta 経由で再生成された証跡を確認
   test -f apps/<app>/src/<bc>/shared/ipc/bindings.ts
   ```

10. **失敗時のリカバリ**:
    - 型 / lint / clippy エラー → **1 回だけ** 自動修正
    - テスト失敗が想定外 → spec を読み直す。1 回だけ patch して再実行
    - specta build 失敗 → `Cargo.toml` の deps と `lib.rs` の `collect_commands![]` 配線を疑う、1 回だけ修正
    - それでも失敗 → 停止して人間に判断を委ねる

11. **完了**:
    ```bash
    bd close ori-impl-green-<slice-id> --reason="all tests green; <N> files added under <source_root>/<bc>/slices/<slice-id>/"
    ```

## 出力例の参照先 (SSoT)

実装コードの worked example は **skill 内に hardcoded で持たず**、以下を参照する (ori-fzr.6 と同方針、DoD 改訂時の sync 漏れ防止):

| 用途 | 参照先 |
| --- | --- |
| pure TS slice の domain / application / infrastructure 形 | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript/example-slice/task-management/slices/complete-task/` |
| Tauri stack の Rust 側 (commands.rs / application.rs / infrastructure.rs / domain.rs) | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/example-slice/rust/task_management/slices/complete_task/` |
| Tauri stack の TS 側 (shared/ipc / shared/test-fixtures / boundary test) | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/example-slice/ts/task-management/` |
| Slice DoD の rule 全文 | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` "Slice Definition of Done" |
| commands.rs Green 条件 (Tauri stack) | `.apm/instructions/ddd-rust.instructions.md` "Slice 完了の必須成果物: commands.rs" section |
| 境界契約宣言の写し方 | `.apm/instructions/feature-spec.instructions.md` "境界契約 section 必須化" |
| production fixture 雛形 | ori-init scaffold (`install-tauri-scaffold.sh` 配置物) — `apps/<app>/src/<bc>/shared/test-fixtures/setupProductionBuilder.ts` |
| `phase_hooks` 由来 specta 再生成 | `architecture.md` frontmatter `phase_hooks.flow-impl-green-post` + `apm-scripts/specta-build.sh` |

## 注意

- **最小実装に徹する**：refactor / abstraction は phase 5 の責務
- **副作用を domain に持ち込まない**：DB / clock / random は interface で抽象化
- **サブ issue を切らない**：checklist 更新で対応
- **テストを書かない**：phase 3 が観点を尽くしている前提。漏れたら phase 3 に戻る
- **`.ori/slices/<id>/` には絶対書かない**：code は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下
- **Tauri stack で commands.rs を skip しない** (DoD rule 2): stub `Err("pending")` を残したまま green とせず、必ず real impl に置換すること。`/ori-doctor` が違反検知する

## 次のアクション

phase 4 完了後、`/ori-flow` 内部なら自動的に phase 5 へ。単独呼び出しの場合：

- **メインパス**：`/ori-refactor <slice-id>` — phase 5。テストを GREEN に保ったまま重複除去・抽象化
- **観点漏れ発覚パス**：実装中に「このケースが spec に無い」と気付いた場合 → phase 3 (`/ori-test-red`) に戻し新観点を追加
- **ドメイン誤り発覚パス**：不変条件が満たせないと気付いた場合 → `/ori-propose` で domain 修正提案
