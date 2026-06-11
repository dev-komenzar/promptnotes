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
- 加えて `.ori/domain/types.md`（meta インデックス、frontmatter + 言語別ファイルへのリンク表）

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
5. **コンパイル確認**：
   - TS: `tsc --noEmit` で型エラーなしを確認
   - Rust: `cargo check`
6. **types.md（meta）の生成**：
   ```markdown
   ## Files

   | aggregate | typescript | rust |
   |-----------|-----------|------|
   | Note | code/typescript/note.ts | code/rust/note.rs |
   ```
7. `bash scripts/lint-domain.sh .ori/domain/types.md` を実行して自己検証
8. lint 失敗時は **1 回だけ** 自動修正、それでもダメなら人間判断

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

## 次のアクション

Phase 10 完了後、ユーザに以下を提示：

- **通常パス**：`/ori-ddd-11a-ui-fields` — 型定義から UI 入力項目を抽出
- **戻る**：型で表現できない概念があるなら `/ori-ddd-5-aggregates` で集約設計を見直す
- **早期実装パス**：UI が無いシステムなら 11a/11b をスキップし、`/ori-ddd-9-workflows` 完了後の slice を直接 `/ori-flow` で実装
