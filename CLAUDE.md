# プロジェクト指示 (AI Agent 向け)

このファイルは本プロジェクトで作業する AI coding agent に向けての指示とコンテキストを提供する。
言語は **日本語** に固定する。Agent の出力も原則として日本語で返答すること。

## 非対話シェルコマンド

ファイル操作時に確認プロンプトで **ハングするのを防ぐため、常に非対話フラグを使用** すること。

`cp`, `mv`, `rm` 等は一部の環境で `-i` (interactive) エイリアスが貼られており、agent が y/n 入力待ちで無限待機になる。

**以下の形式を使用すること:**
```bash
# 確認なしで上書き
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# 再帰操作
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**その他プロンプトが出る可能性のあるコマンド:**
- `scp` - `-o BatchMode=yes` を使用
- `ssh` - `-o BatchMode=yes` を使用 (プロンプトではなく即時失敗にする)
- `apt-get` - `-y` フラグを使用
- `brew` - `HOMEBREW_NO_AUTO_UPDATE=1` 環境変数を使用

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

本プロジェクトは **bd (beads)** を issue tracking に使用する。`bd prime` で全 workflow context とコマンドを確認できる。

### Quick Reference

```bash
bd ready              # 取り組める issue を検索
bd show <id>          # issue 詳細を表示
bd update <id> --claim  # 作業を atomically claim
bd close <id>         # 作業を完了
bd dolt push          # beads data を remote に push
```

### Rules

- **階層化 task tracking**: 詳細は [.claude/rules/task-management.md](.claude/rules/task-management.md) 参照
  - **戦略 (strategic)**: PR 単位の大きな issue / multi-session work / 依存関係 / 永続 context → **bd** で track
  - **戦術 (tactical)**: 現 session 内の linear な実行 step / file 編集レベル → **組み込み task tool (TodoWrite)** で track
  - 判断の core question: "2 週間離れても resume できるか?" YES → bd, NO → TodoWrite
- `bd prime` で詳細なコマンドリファレンスと session close protocol を確認
- 永続知識には `bd remember` を使用 — MEMORY.md は使用しない

## Session Completion

**Work session 終了時**、以下の全 step を完了すること。`git push` が成功するまで作業は完了しない。

**MANDATORY WORKFLOW:**

1. **残作業の issue 化** - follow-up が必要なものを issue 化
2. **Quality gate 実行** (code 変更時) - Test / linter / build
3. **issue status 更新** - 完了 work は close、in-progress は更新
4. **REMOTE に PUSH** - これは必須:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # "up to date with origin" であること
   ```
5. **Cleanup** - stash を clear、remote branch を prune
6. **Verify** - 全変更が commit 済み AND push 済み
7. **Hand off** - 次 session 用に context を提供

**CRITICAL RULES:**
- `git push` が成功するまで作業は完了しない
- push せずに session を終了しない — local に work が孤立する
- "ready to push when you are" と言わない — agent 自身が push すること
- push 失敗時は解決して成功するまで retry すること
<!-- END BEADS INTEGRATION -->

## Task Management (階層構造)

本プロジェクトは **tier 構造** でタスクを管理する。詳細は [.claude/rules/task-management.md](.claude/rules/task-management.md) 参照。

```
bd epic / parent issue        (= 1 PR、bundling 単位)
  ↓
  bd child issue              (= 永続 sub-deliverable、ship 可能単位)
    ↓
    TodoWrite items           (= 1 session 内の実装 step、file 編集レベル)
```

- **bd (strategic)**: multi-session work / 依存関係 / 永続 context / side quest を track
- **TodoWrite (tactical)**: 現 session 内の linear な実行 step を track。複雑化したら bd へ promote

### Epic 化 trigger

- `/ori-flow` 起動時: 即 epic 化
- その他: 推定変更 file 数 ≥ 5 OR 推定変更 package ≥ 2 なら epic 化を提案
- session 跨ぎが発生した時点で epic 化を提案
- 上記に該当しなければ default issue (type=task) で十分

## ルールドキュメント

以下のルールドキュメントが `.claude/rules/` 配下にある。各 file が特定 path pattern に適用される規約を持つ。

| File | 適用先 | 概要 |
|---|---|---|
| [task-management.md](.claude/rules/task-management.md) | (全局) | bd と TodoWrite の tier 構造、epic 化 trigger、lazy promote、dispatch rule |
| [ori-conventions.md](.claude/rules/ori-conventions.md) | `.ori/**/*.md` | H2/H3 見出しID必須、派生文書の保護、`/ori-sync` 呼出 |
| [domain-phase.md](.claude/rules/domain-phase.md) | `.ori/domain/**/*.md` | Phase 別 domain doc 構造規約 (aggregates / bounded-contexts / domain-events) |
| [feature-manifest.md](.claude/rules/feature-manifest.md) | `.ori/features/*/manifest.yaml` | manifest 必須フィールド、rename 禁止、derives_from 指定 |
| [feature-spec.md](.claude/rules/feature-spec.md) | `.ori/features/*/spec.md` | 派生文書のため直接編集禁止、`/ori-sync --force` で proposal 生成 |
| [ddd-typescript.md](.claude/rules/ddd-typescript.md) | `apps/*/src/**/*.ts` | TS DDD モジュール構造、BC 別 layout、VSA 命名 |
| [ddd-rust.md](.claude/rules/ddd-rust.md) | `src/**/*.rs` | Rust VO newtype pattern、thiserror、Tauri command 規約 |
| [ddd-test.md](.claude/rules/ddd-test.md) | `**/*.{spec,test}.{ts,tsx}` | vitest、`describe('feature:<id>')`、spec.md セクション参照、fast-check |
| [ui-test.md](.claude/rules/ui-test.md) | `**/*.{spec,test}.tsx` 等 | UI selector 規約、getByRole/getByLabelText 第一推奨 |

## Build & Test

### Dev shell

Nix flake で toolchain を固定。`nix develop` または `direnv allow` (`.envrc` 存在) で必要な Rust / Bun / Node / cargo-tauri / system libs が入った shell に入る。

```bash
nix develop          # OR: direnv allow して cd
```

### Frontend (SvelteKit + Vite + Vitest)

package manager は **bun** (`bun.lock`)。コマンドは `apps/promptnotes/` 配下で実行。

```bash
cd apps/promptnotes
bun install                       # 依存 install

bun run dev                       # 開発 server (Vite)
bun run build                     # production build (frontend のみ)
bun run preview                   # production build の preview

bun run check                     # svelte-check で型 check
bun run lint                      # prettier --check + eslint
bun run format                    # prettier --write

bun run test                      # vitest --run (1 shot)
bun run test:unit                 # vitest (watch)
```

Vitest は 2 project を持つ (vite.config.ts):
- `client`: `src/**/*.svelte.{test,spec}.{js,ts}` — Playwright chromium headless で browser component test
- `server`: `src/**/*.{test,spec}.{js,ts}` (`.svelte.*` 以外) — node env で logic test

### Tauri backend (Rust)

Tauri app crate は `apps/promptnotes/src-tauri/`。proptest / tempfile が dev-deps に入っている。

```bash
cd apps/promptnotes
bun run tauri dev                 # Tauri 開発 (frontend + Rust hot reload)
bun run tauri build               # Tauri production build (frontend build → cargo build --release → bundle)
bun run tauri build --no-bundle   # binary のみ (Nix package 等で wrap する場合)

cd src-tauri
cargo test                        # Rust unit / proptest 一括
cargo test --release              # release build で test
cargo clippy --all-targets        # lint
cargo fmt --check                 # format check
```

### 統合 build (Nix)

```bash
nix build                         # flake の packages.default (= cargo tauri build --no-bundle + wrap)
nix run                           # build 済み binary を起動
```

### Domain reference types (`.ori/domain/code/rust/`)

Phase 10 の reference 型定義。production コードではなく `.ori` ドメイン文書の一部。変更時は `cargo test` で compile 可否を確認。

```bash
cd .ori/domain/code/rust
cargo test                        # compile check を兼ねる
```

## Architecture Overview

Tauri v2 desktop app。frontend は SvelteKit (static adapter、SSR 無し)、backend は Rust (Tauri command)。

- `apps/promptnotes/src/` — SvelteKit frontend (`$lib/`, route ページ)
- `apps/promptnotes/src-tauri/` — Tauri v2 Rust backend (Tauri command / domain logic)
- `.ori/` — DDD distill 文書 (Phase 1-11) と reference 型定義
- `.claude/rules/` — path pattern 別の lint / 構造規約 (上記ルールドキュメント表参照)
- `flake.nix` — NixOS 対応 devShell + native package build

## Conventions & Patterns

- DDD / VSA (Value-Oriented SE) パターン。BC 別 module layout は [.claude/rules/ddd-typescript.md](.claude/rules/ddd-typescript.md) / [.claude/rules/ddd-rust.md](.claude/rules/ddd-rust.md) 参照
- Test 規約は [.claude/rules/ddd-test.md](.claude/rules/ddd-test.md) / [.claude/rules/ui-test.md](.claude/rules/ui-test.md) 参照
- VO の Smart Constructor は proptest / fast-check で fuzz
- Mock は adapter 境界のみ。domain 純粋コードは実物使用