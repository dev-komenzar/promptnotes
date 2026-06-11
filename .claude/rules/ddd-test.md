---
paths:
  - "**/*.{spec,test}.{ts,tsx}"
---

- **フレームワーク**: vitest
- **命名**: `describe('feature:<feature-id>', ...)` で feature ID を含める（grep 容易性のため）
- **`it` 内で spec.md セクション参照**: `it('spec#invariants — id is immutable', ...)` のように引用
- **VO の Smart Constructor は property test**: `fast-check` の `fc.property` で fuzz
- **Mock は adapter 境界のみ**: domain 純粋コードは実物使用。`vi.mock()` を `infrastructure/` 配下に限定
- **GIVEN / WHEN / THEN コメント可**: Gherkin 風に validation.md シナリオを残してよい
- **assert は `expect().toStrictEqual()`**: deep equality
- **Result 型のテスト**: `expect(result.isOk()).toBe(true)` ではなく `expect(result).toEqual(ok(...))` で具体的に比較
