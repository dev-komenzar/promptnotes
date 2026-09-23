# テスト共通規約（メタルール）

> 本ファイルは **言語・テストライブラリ中立**なテストのメタルールのみを担う。
> ランナー名・property test ライブラリ・assertion 記法などの concretion は
> **正典 (pattern / stacks) に一本化済み**であり、ここではポインタのみを示す。
> concretion を二重管理しないこと。

## メタルール

- **spec.md をトレースする**: `describe` / `it` 名に `spec.md#<section-id>` を引用し、
  domain 文書とテストの相互 grep を可能にする
- **feature / scenario ID を最外殻に**: `describe('slice:<slice-id>', ...)` /
  `describe('scenario:<scenario-id>', ...)` のように該当 ID を最外殻 `describe` に
  置く
- **Mock / fake は adapter 境界のみ**: domain 純粋コードは実物使用。mock 注入は
  `infrastructure/` 配下に限定し、slice DoD test では行わない
- **副作用 (clock / fs 等) は引数注入 / trait 抽象で**: domain/application に直書き
  しない
- **GIVEN / WHEN / THEN コメント可**: Gherkin 風に validation.md シナリオを残してよい
- **DoD boundary test は外部境界経由のみ**: `application/` / `infrastructure/` への
  直 import は DoD 違反（rule 2）。fixture は production wiring（rule 3）

## 正典ポインタ

| 関心事 | 正典 |
| --- | --- |
| stack-agnostic メタルール + UI selector / testid 規約 | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` "Test conventions" |
| TypeScript のランナー / assertion / property test | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript/test.md` |
| TypeScript-Tauri の boundary test / production fixture | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/test.md` |
| Rust の言語共通規約 / Tauri command surface | `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/rust/test.md` |

## 責務分離

- 本ファイル（メタルール）は**全テスト層**（unit / UI / scenario=E2E）共通の
  「テストの心構え」。
- DOM のクエリ方法は `ui-test` へ、シナリオ E2E の構造は `scenario-test` へ
  分離済み。重複する内容は持たない。
