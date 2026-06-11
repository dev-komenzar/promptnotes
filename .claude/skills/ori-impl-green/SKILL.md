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
  - `.ori/slices/<id>/manifest.yaml`（`bc:` と `app:` の解決に必要）
  - `.ori/config.yaml`（`workspace.apps:` から `app:` 解決、fallback として `apps[].path`/src を `<source_root>` に使う）
  - `.ori/architecture.md`（あれば `root.path` / `roots[<id>].path` を canonical な `<source_root>` として優先採用）
  - `<source_root>/<bc>/slices/<slice-id>/tests/*.test.ts`（phase 3 で RED 確認済み）
  - `.apm/instructions/ddd-typescript.instructions`（実装規約）
- 出力：
  - `<source_root>/<bc>/slices/<slice-id>/domain/...`
  - `<source_root>/<bc>/slices/<slice-id>/application/...`
  - `<source_root>/<bc>/slices/<slice-id>/infrastructure/...`（必要時）
  - `<source_root>/<bc>/slices/<slice-id>/tests/` 配下の全テストが GREEN

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
   - `pnpm -F <app> test <base>/tests` を Bash で実行し RED であることを確認（phase 3 完了の検証）
   - 既に GREEN なら停止し phase 3 へ差し戻す（`/ori-test-red` の "GREEN-on-first-run" と同等）
2. **テストを 1 本ずつ通す**：
   - 一番外側の `it` から順に attack
   - 「テストを通すための最小限のコード」だけ書く（YAGNI）
   - 関連する VO / entity / workflow ステップを `<base>/domain/` に追加
   - I/O が必要なら `<base>/infrastructure/` に repository 実装を追加し、`<base>/application/` で DI
3. **層配置のチェック**：
   - `domain/` に I/O 依存がないか
   - 集約をまたぐ呼び出しが repository interface 経由か
   - branded types が裸の primitive で漏れていないか
4. **進捗の記録**：beads issue description の checklist を Bash で更新：
   ```bash
   bd update ori-impl-green-<slice-id> --notes="step N done: <topic>"
   ```
   - **サブ issue は切らない**（ori-flow.md 注意事項）
5. **全テスト GREEN を確認**：
   ```bash
   pnpm -F <app> test <base>/tests
   pnpm -F <app> typecheck
   ```
6. **lint / format**：
   ```bash
   pnpm lint --fix
   pnpm format
   ```
7. **出力先の self-check**：
   ```bash
   # 禁止 path への漏出が無いことを確認
   test ! -d .ori/slices/<slice-id>/src || (echo "ERROR: .ori/slices/<id>/src must not exist" && exit 1)
   ```
8. **失敗時のリカバリ**：
   - 型 / lint エラー → **1 回だけ** 自動修正
   - テスト失敗が想定外 → spec を読み直す。1 回だけ patch して再実行
   - それでも失敗 → 停止して人間に判断を委ねる
9. **完了**：
   ```bash
   bd close ori-impl-green-<slice-id> --reason="all tests green; <N> files added under <source_root>/<bc>/slices/<slice-id>/"
   ```

## 出力テンプレート

```ts
// apps/<app>/src/note-capture/slices/capture-auto-save/domain/vo/note-body.ts
import { Result, ok, err } from '@ori-ori/result';

export type NoteBody = string & { readonly __brand: 'NoteBody' };

export const NoteBody = {
  create(raw: string): Result<NoteBody, 'EmptyBody'> {
    return raw.trim() === '' ? err('EmptyBody') : ok(raw as NoteBody);
  },
};
```

```ts
// apps/<app>/src/note-capture/slices/capture-auto-save/application/capture-auto-save.ts
import { NoteBody } from '../domain/vo/note-body';
import type { NoteRepository } from '../domain/note-repository';
import type { Clock } from '../domain/clock';
import { Result, ok, err } from '@ori-ori/result';

export type CaptureAutoSaveCommand = {
  noteId: string;
  body: string;
  occurredAt: Date;
};

export const captureAutoSave =
  (deps: { repo: NoteRepository; clock: Clock }) =>
  async (cmd: CaptureAutoSaveCommand): Promise<Result<NoteSaved, DomainError>> => {
    const body = NoteBody.create(cmd.body);
    if (body._tag === 'Left') return err('EmptyBody');
    // ...
  };
```

## 注意

- **最小実装に徹する**：refactor / abstraction は phase 5 の責務
- **副作用を domain に持ち込まない**：DB / clock / random は interface で抽象化
- **サブ issue を切らない**：checklist 更新で対応
- **テストを書かない**：phase 3 が観点を尽くしている前提。漏れたら phase 3 に戻る
- **`.ori/slices/<id>/` には絶対書かない**：code は必ず `<source_root>/<bc>/slices/<slice-id>/` 配下

## 次のアクション

phase 4 完了後、`/ori-flow` 内部なら自動的に phase 5 へ。単独呼び出しの場合：

- **メインパス**：`/ori-refactor <slice-id>` — phase 5。テストを GREEN に保ったまま重複除去・抽象化
- **観点漏れ発覚パス**：実装中に「このケースが spec に無い」と気付いた場合 → phase 3 (`/ori-test-red`) に戻し新観点を追加
- **ドメイン誤り発覚パス**：不変条件が満たせないと気付いた場合 → `/ori-propose` で domain 修正提案
