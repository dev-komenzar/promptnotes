---
name: ori-refactor
description: /ori-flow phase 5。テスト GREEN を保ったまま重複除去・命名整理を行う。新機能追加禁止。capability=fast で安価な model を割当
---

ユーザが `/ori-refactor <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 5 として起動した際に、**phase 4 で書いた実装を tidy する**。**新しい振る舞いは追加しない**。テストが常に GREEN を保つことが唯一の合格条件。

## 引数

- `slice-id`：対象 slice の id

## 役割

- **コード整理者**：重複除去・命名改善・小さな抽象化
- **テストガード**：refactor 前後でテスト結果が等しいことを確認
- **スコープ守護者**：新機能・新テスト・spec 変更は **禁止**（別 phase の責務）

## capability 設定

- `ori-refactor` agent の capability は **`fast`**（安価な model で十分）
- 創造性より「規則的な変換」が支配的なため

## 入力 / 出力

- 入力：
  - `<source_root>/<bc>/slices/<slice-id>/{domain,application,infrastructure,presentation}/`（phase 4 の出力）
  - `<source_root>/<bc>/slices/<slice-id>/tests/`（GREEN 状態、変更禁止）
  - `.ori/slices/<id>/manifest.yaml`（`bc:` と `app:` の解決）
  - `.ori/config.yaml`（`workspace.apps:`、fallback `<source_root>` 解決）
  - `.ori/architecture.md`（あれば `root.path` / `roots[<id>].path` を canonical `<source_root>` として優先採用）
  - `.apm/instructions/ddd-typescript.instructions`
- 出力：
  - 同じ `<source_root>/<bc>/slices/<slice-id>/...` を tidy（差分のみ）
  - テストファイル：変更禁止（新観点追加は phase 3 へ戻る）

## `<app>` `<bc>` `<source_root>` の解決

ori-impl-green と同一ロジック:

1. **`<bc>`**：`.ori/slices/<id>/manifest.yaml` の `bc:` field
2. **`<app>`**：manifest.`app:` → なければ `.ori/config.yaml` `workspace.apps:`（1 個なら採用、N 個ならエラー）
3. **`<source_root>`**：`.ori/architecture.md` `root.path`（または `roots[<id>].path`）→ なければ `workspace.apps[<app>].path + "/src"` で fallback
4. **slice base**：`<source_root>/<bc>/slices/<slice-id>/`

## やってよいこと（scope）

| 変換 | 例 |
|------|----|
| 重複除去 | 同じヘルパ関数の重複を 1 つに |
| 命名改善 | `doWork` → `applyAutoSave` |
| 関数抽出 | 大きな関数を意味単位で分割 |
| 早期 return | ネスト解消 |
| import 整理 | 順序、不要 import 削除 |
| 型強化 | `any` → 具体型、`as` → smart constructor |
| 純粋関数化 | 副作用を boundary に押し出す |

## やってはいけないこと（out of scope）

- 新しいテストの追加（→ phase 3 へ戻る）
- 新しいパブリック関数の追加（→ phase 4 へ戻る）
- spec.md / domain 文書の編集（→ `/ori-propose`）
- パッケージ間境界の変更（独立 slice にすべき）
- パフォーマンス最適化のためのアルゴリズム変更（observable 振る舞いが変わるなら別 issue）

## 手順

1. **前提確認**：
   - `pnpm -F <slice-pkg> test` が GREEN であることを確認
   - 既に RED なら停止し phase 4 へ差し戻し
2. **diff 抽出**：phase 4 の commit から「いま自分が触れた範囲」を `git diff HEAD~N` で確認
3. **smell 検出**：
   - 同じパターンが 3 箇所以上 → 抽出候補
   - 関数 > 50 行 / cyclomatic > 7 → 分割候補
   - 命名が不明瞭（`data`, `tmp`, `do_*`） → リネーム候補
4. **小さく刻む**：1 リファクタ 1 commit を推奨。各ステップ後に：
   ```bash
   pnpm -F <slice-pkg> test
   pnpm -F <slice-pkg> typecheck
   ```
   を実行し GREEN 維持を確認
5. **失敗時**：
   - test が RED → **即座に revert**（`git checkout -- .` で対象ステップを破棄）
   - typecheck 失敗 → **1 回だけ** 自動修正、それでも駄目なら revert
6. **完了確認**：
   ```bash
   pnpm lint --fix
   pnpm format
   pnpm -F <slice-pkg> test
   ```
7. **完了**：
   ```bash
   bd close ori-refactor-<slice-id> --reason="tidy done; <N> refactor commits; tests still green"
   ```

## 注意

- **テストを書かない**：観点漏れに気付いたら phase 3 へ戻る
- **振る舞いを変えない**：API 入出力 / event payload を変えたら別 phase
- **小さく commit**：1 リファクタ 1 commit。失敗時の rollback コスト最小化
- **新機能追加は禁止**：「ついでに直す」は脱線。別 issue を切る

## 次のアクション

phase 5 完了後、`/ori-flow` 内部なら自動的に phase 6 へ。単独呼び出しの場合：

- **メインパス**：`/ori-review <slice-id>` — phase 6。fresh-context で reviewer agent が adversarial レビュー
- **観点漏れ発覚パス**：refactor 中に「このケースが test に無い」と気付いた場合 → `/ori-test-red` へ戻る
- **大きな構造変更が必要パス**：refactor の範疇を超える場合は新 slice を作成
