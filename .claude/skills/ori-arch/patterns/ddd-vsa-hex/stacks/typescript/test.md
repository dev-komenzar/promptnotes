# Stack: TypeScript — Test conventions

> pattern:ddd-vsa-hex × stack:typescript のテスト concretion 正典。
> stack-agnostic なメタルールは `../pattern.md` "Test conventions" を参照。
> 本ファイルは **ランナー / property test ライブラリ / assertion 記法 / 配置**
> などの stack-specific な具体のみを担う。

## テストランナー / ライブラリ

- **フレームワーク**: vitest
- **property test**: fast-check（VO の Smart Constructor を `fc.property` で fuzz）

## 命名 / トレーサビリティ

- **feature ID を最外殻に**: `describe('feature:<feature-id>', ...)`（`../pattern.md`
  "Test conventions" の `slice:<slice-id>` / `scenario:<scenario-id>` 規約を TS
  で踏襲。他 stack と衝突しない範囲で id 接頭辞を合わせる）
- **`it` 内で spec.md セクション参照**: `it('spec#invariants — id is immutable', ...)`
  のように引用する

## assertion 記法

- **assert は `expect().toStrictEqual()`**: deep equality で比較
- **Result 型のテスト**: `expect(result.isOk()).toBe(true)` ではなく
  `expect(result).toEqual(ok(...))` で具体的に比較

## 配置 / 責務

- **slice tests の配置**: `<source_root>/<bc>/slices/<slice-id>/tests/`（impl と
  co-locate）
- **domain / application unit test**: sibling import 可
  (`../application/handler.ts`, `../domain/vo/<name>.ts`)
- **DoD boundary test**: `bindings` 経由のみ（`../pattern.md` rule 2/3、
  `ddd-typescript.instructions.md` の "#test-binding-only" 参照）
- **internal fake/mock unit test**: `application/` 内に co-locate 可。DoD カウント
  から除外（rule 3）

## 参考

- 参照実装（test ファイル）: `example-slice/task-management/slices/complete-task/tests/*.test.ts`
- 実装側規約: `ddd-typescript.instructions.md`
