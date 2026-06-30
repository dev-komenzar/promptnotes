---
name: ori-ddd-10-types
description: distill-ddd Phase 10（Types）。aggregate / workflow を言語別の compile 可能な型定義へ落とす（TS / Rust など）
---

ユーザが `/ori-ddd-10-types` を呼んだ際、distill-ddd Phase 10（Types）を ori convention 注入版で実行します。**aggregate / workflow / event を、選択した言語の compile 可能な型定義に落とし込み**、`.ori/domain/code/` 配下に保存します。

## 役割

- **型設計者**：DDD の概念を言語固有の型システムに翻訳（branded type, smart constructor, Result）
- **言語切替者**：プロジェクトで選択中の言語に対応（TS / Rust / Kotlin / Scala / F# / C#）
- **記録係**：`.ori/domain/code/<lang>/<topic>.<ext>` に保存

## 入力 / 出力

- 入力：`.ori/domain/aggregates.md`（Phase 5）、`.ori/domain/domain-events.md`（Phase 6）、`.ori/domain/workflows/`（Phase 9 が先行している場合）
- 出力：`.ori/domain/code/<lang>/` 配下に型定義ファイル
  - 例：`.ori/domain/code/typescript/note.ts`、`.ori/domain/code/rust/note.rs`
  - **これは domain 文書側のコード資産**。`src/` には phase 4 で写経／参照
- 加えて `.ori/domain/types.md`（meta インデックス、`ori:` frontmatter + 言語別ファイルへのリンク表）
  - `node_id: type-definitions:index`
  - `type: type-definitions`
  - `depends_on: [aggregate:collection, event:collection, workflow:index]`
  - `modules: [<lang>/<topic>.<ext> の相対 path 列]`（design.md §5 の `modules:` 任意 field で code と紐付け）

## 言語と DDD パターンの写像

| 概念 | TypeScript | Rust |
|------|-----------|------|
| Branded type | `type NoteId = string & { readonly __brand: 'NoteId' }` | `pub struct NoteId(String);` |
| Smart constructor | `NoteBody.create(raw): Result<NoteBody, E>` | `impl NoteBody { pub fn try_from(raw: String) -> Result<Self, E> }` |
| Sum type | discriminated union | `enum` |
| Result | `import { Result } from '@ori-ori/result'` | `Result<T, E>` (stdlib) |
| Workflow | `(deps) => (input) => Promise<Result<...>>` | `fn(deps) -> impl Fn(Input) -> Result<...>` |

## 手順

1. **前提確認**：
   - 対象言語をユーザに確認（project lang 設定を見る）
   - `.ori/domain/code/<lang>/` が既にあれば一覧表示し「追記 / 上書き / 中断」を選ばせる
2. **集約 → 型**：
   - aggregate の 構成要素 / 不変条件 / 公開操作 を型として表現
   - VO は smart constructor 経由でしか作れないように `private` constructor + factory pattern
3. **event → 型**：discriminated union（TS）or enum（Rust）として
4. **workflow → 型シグネチャ**：dependencies injection 形式の関数型
5. **build artifact 用 `.gitignore` を先に書く** ⚠️ **コンパイル前に必須**：
   - `.ori/domain/code/<lang>/.gitignore` が無ければ言語ごとの default を書き出す（既にあれば skip / 不足パターンを追記）
   - 目的：次手順の `cargo check` / `tsc --noEmit` 等が生成する build artifact (`target/`, `node_modules/`, `*.tsbuildinfo`, `__pycache__/` …) を **commit に巻き込まない**
   - **順序厳守**：先に compile して artifact を生んでから `.gitignore` を書くと、untracked artifact が `git add .` 等で漏れて入る事故が起きる（実例：`.ori/domain/code/rust/target/` を 596 files commit）
   - 言語別 default（無ければ作る）：
     ```gitignore
     # .ori/domain/code/rust/.gitignore
     /target/
     **/*.rs.bk
     Cargo.lock        # domain 側コードは lib なので lock は無視（bin にしたいなら外す）
     ```
     ```gitignore
     # .ori/domain/code/typescript/.gitignore
     node_modules/
     dist/
     *.tsbuildinfo
     ```
     ```gitignore
     # .ori/domain/code/kotlin/.gitignore
     build/
     .gradle/
     *.class
     ```
     ```gitignore
     # .ori/domain/code/python/.gitignore
     __pycache__/
     *.pyc
     .venv/
     dist/
     *.egg-info/
     ```
6. **コンパイル確認**：
   - TS: `tsc --noEmit` で型エラーなしを確認
   - Rust: `cargo check`
   - **commit 前 sanity check**：`git status .ori/domain/code/<lang>/` で `target/` / `node_modules/` 等が untracked に出てこないこと（出ていたら step 5 の `.gitignore` が漏れている → 修正して再確認）
7. **types.md（meta）の生成**：
   ```markdown
   ---
   ori:
     node_id: type-definitions:index
     type: type-definitions
     depends_on:
       - aggregate:collection
       - event:collection
       - workflow:index
     modules:
       - code/typescript/note.ts
       - code/rust/note.rs
   ---

   # Type Definitions {#type-definitions}

   ## Files {#files}

   | aggregate | typescript | rust |
   |-----------|-----------|------|
   | Note | code/typescript/note.ts | code/rust/note.rs |
   ```
8. `bash ./scripts/lint-domain.sh .ori/domain/types.md` を実行して自己検証
9. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間判断

## 出力テンプレート（TypeScript）

```ts
// .ori/domain/code/typescript/note.ts
import { Result, ok, err } from '@ori-ori/result';

export type NoteId = string & { readonly __brand: 'NoteId' };
export const NoteId = {
  of(raw: string): Result<NoteId, 'InvalidNoteId'> {
    return /^note-[a-z0-9-]+$/.test(raw) ? ok(raw as NoteId) : err('InvalidNoteId');
  },
};

export type NoteBody = string & { readonly __brand: 'NoteBody' };
export const NoteBody = {
  create(raw: string): Result<NoteBody, 'EmptyBody'> {
    return raw.trim() === '' ? err('EmptyBody') : ok(raw as NoteBody);
  },
};

export type Note = {
  readonly id: NoteId;
  readonly body: NoteBody;
  readonly createdAt: Date;
  readonly updatedAt: Date;
};

export type DomainEvent =
  | { type: 'NoteSaved'; noteId: NoteId; bodyHash: string; occurredAt: Date }
  | { type: 'NoteEmptied'; noteId: NoteId; occurredAt: Date };
```

## 注意

- **これは domain 側の型定義**：`src/` のアプリケーションコードではない
- **コンパイルが通らない型は書かない**：phase 10 は「動く型」が成果物
- **VO は smart constructor 必須**：直接 new / cast を許可しない
- **言語複数化はオプション**：MVP では TS のみで OK
- **build artifact を commit に巻き込まない**：`cargo check` / `npm install` / `tsc` 等は必ず `.gitignore` を先に書いてから実行する（手順 step 5）。`target/` / `node_modules/` / `build/` を一度 commit してしまうと、push 後の巻き戻しは履歴汚染になるため**事故予防が第一**

## 次のアクション

Phase 10 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-11a-ui-fields` — 型定義から UI 入力項目を抽出
- **戻る**：型で表現できない概念があるなら `/ori-ddd-5-aggregates` で集約設計を見直す
- **早期実装パス**：UI が無いシステムなら 11a/11b をスキップし、`/ori-ddd-9-workflows` 完了後の slice を直接 `/ori-flow` で実装
  - 前提：`.ori/architecture.md` 必須。未生成なら **先に `/ori-arch`** を実行して pattern / stack を確定する
