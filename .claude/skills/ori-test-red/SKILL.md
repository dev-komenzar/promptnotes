---
name: ori-test-red
description: /ori-flow phase 3。spec.md のテスト観点から failing test を <source_root>/<bc>/slices/<id>/tests/ に書き起こす（RED 確認まで。<source_root> は `.ori/architecture.md` root.path または `apps[].path`/src で resolve）
---

ユーザが `/ori-test-red <slice-id>` を呼ぶ、または `/ori-flow` 内部から phase 3 として起動した際に、**該当 slice の `<source_root>/<bc>/slices/<slice-id>/tests/` 配下に failing test を書く**。**impl は書かない**。RED が観測できた時点で完了。`<source_root>` は `.ori/architecture.md` の `root.path`（単一 root）または `roots[<id>].path`（multi-root）、なければ `.ori/config.yaml` `workspace.apps[<app>].path + "/src"` で resolve（後述）。

## 引数

- `slice-id`：対象 slice の id（`.ori/slices/<id>/spec.md` が存在する事を前提）

## 役割

- **テスト設計者**：spec.md `## テスト観点` を vitest テストに展開
- **プロパティテスター**：value object の smart constructor は fast-check で生成テスト
- **トレーサビリティ守護者**：`it` の説明は spec.md の該当セクション id を引用する
- **品質ゲート**：最初の実行で **すべて GREEN** になったら、それは spec / impl のどちらかにバグの兆候 → **強制停止**

## 入力 / 出力

- 入力：
  - `.ori/slices/<id>/spec.md`（phase 1 で生成済み）
  - `.ori/slices/<id>/manifest.yaml`（`bc:` と `app:` の解決に必要）
  - `.ori/config.yaml`（`workspace.apps:` から `app:` 解決、fallback として `apps[].path`/src を `<source_root>` に使う）
  - `.ori/architecture.md`（あれば `root.path` / `roots[<id>].path` を canonical な `<source_root>` として優先採用）
  - `.apm/instructions/ddd-test.instructions`（テスト規約）
- 出力：
  - `<source_root>/<bc>/slices/<slice-id>/tests/<topic>.test.ts`（1 観点 1 ファイル基本、関連は集約可）
  - test runner: vitest
  - property test: fast-check（VO の smart constructor）

## `<app>` `<bc>` `<source_root>` の解決

skill 起動時に以下の順序で resolve:

1. **`<bc>`**：`.ori/slices/<id>/manifest.yaml` の `bc:` field
2. **`<app>`**：
   - manifest に `app:` field があれば優先採用
   - なければ `.ori/config.yaml` の `workspace.apps:` を参照
     - 要素 1 個 → その entry を採用
     - 要素 N 個 → エラー停止（manifest に `app:` を追加するよう user に促す）
   - config 未存在 → `/ori-init` 未実行エラー
3. **`<source_root>`**（test を書く base directory）：
   - **優先**: `.ori/architecture.md` が存在し `root.path`（単一 root）または `roots[<id>].path`（multi-root、manifest の `root:` field で選択）が設定されていればそれを採用
   - **fallback**: `.ori/architecture.md` 未生成なら `<workspace.apps[<app>].path>/src`（典型: `apps/<app>/src`）
   - **brownfield 例**: 既存 monorepo の `promptnotes/` subdir に `.ori/` を被せた場合、`workspace.apps[0].path: promptnotes` を設定すれば `<source_root>` は `promptnotes/src` に解決される
4. **slice base**：`<source_root>/<bc>/slices/<slice-id>/`（出力先 path はこれを固定値として組み立てる）

## ddd-test.instructions 準拠ルール

| ルール | 内容 |
|--------|------|
| runner | vitest |
| 命名 | `describe('slice:<slice-id>', ...)` を必ず最外殻に |
| it 引用 | `it('spec.md#<section-id>: <観点>', ...)` 形式で spec へのリンクを残す |
| VO テスト | smart constructor は fast-check の `fc.property` で網羅 |
| import | テスト対象は **sibling import**（`../application/handler.ts`, `../domain/vo/<name>.ts`）。impl 不在でも `// @ts-expect-error` で意図的に止めない |
| skip 禁止 | `it.skip` / `.todo` を使わない。書くなら失敗させる |

## 禁止事項

- **`.ori/slices/<id>/src/` への出力は禁止**。`.ori/slices/<id>/` は SSoT メタ専用（manifest.yaml / spec.md / status.yaml / notes.md 等のみ）。code および tests は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下に置く
- 配置先は skill 起動時に Bash で `mkdir -p <source_root>/<bc>/slices/<slice-id>/tests` を実行し、出力先を強制的に resolve 済み path に固定する

## 手順

1. **前提確認**：
   - `.ori/slices/<id>/spec.md` の `## テスト観点 {#test-perspectives}` を Read
   - 観点が空 / TBD のみなら停止し「先に `/ori-derive` で spec を埋めるか、ユーザに観点を確認」
   - manifest.yaml と `.ori/config.yaml` / `.ori/architecture.md` から `<app>` `<bc>` `<source_root>` を resolve
2. **テスト観点を列挙**：spec.md から bullet を抽出し、各観点を 1 つの `it` に対応付ける
3. **テストファイルの構成**：
   ```
   <source_root>/<bc>/slices/<slice-id>/tests/
     <slice-id>.test.ts             ← happy path + 主要観点
     <slice-id>-vo.property.test.ts ← VO smart constructor の fast-check（必要時）
     <slice-id>-edge.test.ts        ← edge case を集約（任意）
   ```
4. **テストを書く**（impl は書かない）：
   - 対象 src は **sibling** から import（`../application/handler.ts`, `../domain/vo/<name>.ts`）
   - impl 不在の段階では module-not-found / type error で fail。`// @ts-expect-error` は不要、失敗をそのまま観測
5. **`pnpm test --filter <slice-id>` 相当を Bash で実行**して RED を確認：
   ```bash
   pnpm -F <app> test <source_root>/<bc>/slices/<slice-id>/tests
   ```
6. **GREEN 観測時：強制停止**
   - 最初から全テストが GREEN なら spec か impl のどちらかにバグ可能性が高い
   - bd issue にコメントを残して human flag：
     ```bash
     bd update ori-test-red-<slice-id> --notes="test was GREEN at first run — spec gap suspected"
     bd human ori-test-red-<slice-id> --reason="GREEN-on-first-run anomaly"
     ```
   - ユーザに「spec の観点が impl 済みの動作と合致しているか確認してください」と促す
7. **RED 観測時：phase 3 完了**
   - 失敗テストの一覧を beads notes に記録：
     ```bash
     bd update ori-test-red-<slice-id> --notes="N failing tests written: ..."
     ```
   - `bd close ori-test-red-<slice-id>` で完了
8. lint / format：`pnpm lint --fix` を最後に走らせる

## 失敗時のリカバリ

- テストファイル自体に文法エラー → **1 回だけ** 自動修正
- それでも失敗 → 停止して人間に判断を委ねる

## 出力テンプレート

```ts
// apps/<app>/src/note-capture/slices/capture-auto-save/tests/capture-auto-save.test.ts
import { describe, it, expect } from 'vitest';
import { captureAutoSave } from '../application/capture-auto-save';

describe('slice:capture-auto-save', () => {
  it('spec.md#test-perspectives: happy path → NoteSaved event', async () => {
    const result = await captureAutoSave({
      noteId: 'note-1',
      body: 'hello',
      occurredAt: new Date('2026-05-14T00:00:00Z'),
    });
    expect(result).toMatchObject({ type: 'NoteSaved' });
  });

  it('spec.md#test-perspectives: empty body → not persisted', async () => {
    const result = await captureAutoSave({
      noteId: 'note-1',
      body: '   ',
      occurredAt: new Date('2026-05-14T00:00:00Z'),
    });
    expect(result).toMatchObject({ type: 'EmptyBody' });
  });
});
```

```ts
// apps/<app>/src/note-capture/slices/capture-auto-save/tests/capture-auto-save-vo.property.test.ts
import { describe, it } from 'vitest';
import fc from 'fast-check';
import { NoteBody } from '../domain/vo/note-body';

describe('slice:capture-auto-save VO', () => {
  it('spec.md#invariants: NoteBody rejects whitespace-only', () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 1, maxLength: 50 }).filter((s) => s.trim() === ''),
        (input) => NoteBody.create(input)._tag === 'Left',
      ),
    );
  });
});
```

## 注意

- **impl を書かない**：型シグネチャを `../application/` 等に想像で書きたくなっても禁止。phase 4 の責務
- **観点ごとに 1 it**：1 つの it に複数 expect を詰めない
- **GREEN-on-first-run は赤信号**：spec or impl にバグの兆候。安易にテスト追加で済まさない
- **`.ori/slices/<id>/` には絶対書かない**：code と tests は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下

## 次のアクション

phase 3 完了後、`/ori-flow` 内部なら自動的に phase 4 へ。単独呼び出しの場合：

- **メインパス**：`/ori-impl-green <slice-id>` — phase 4。失敗テストを GREEN にする最小実装
- **GREEN-on-first-run で停止した場合**：
  - 観点漏れなら spec を見直し `/ori-derive` で再派生
  - impl が予期せず存在するなら `git log` で来歴を確認し、`/ori-doctor` で整合性検査
- **戻る**：spec の観点が貧弱なら `/ori-plan` で TBD を詰めるか domain に遡る
