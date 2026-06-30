---
ori:
  node_id: pattern:ddd-vsa-hex/ai-notes
  type: pattern-ai-notes
  version: 1.0.0
  applies_to: pattern:ddd-vsa-hex
---

# AI notes — DDD-VSA-Hex

このファイルは AI agent が `/ori-flow new-slice` や `/ori-impl-green` 等で
**新しい slice を生成 / 修正する際の行動指示**。pattern.md は人間 + AI 共通の
概念定義、ai-notes.md は AI 専属の "do / don't" を書く。

## AI considerations

### Always do

1. **`pattern.md` の Dependency rules に従う**。
   slice 内では `presentation → application → domain` および
   `infrastructure → domain` の一方向 pipeline、cross-slice 直 import は禁止。
   slice の public entry (`index.ts` / `mod.rs` 等)経由でのみ外に晒す。

2. **`stacks/<stack>/example-slice/` を必ず参照する**。
   新しい slice を生成する前に、現在の stack に対応する `example-slice/` を読み、
   ファイル配置と命名規則を **そのまま踏襲する** こと。例 slice はユーザの
   ドメインに置き換える「型紙」として使う。

3. **Slice id をユーザのドメインに合わせる**。
   example-slice の `complete-task` をコピーして生成 file 名に残してはいけない。
   slice id は `<verb>-<noun>` で use case を表現 (`create-order`, `archive-task`,
   `assign-reviewer` 等)。

4. **branded VO は domain/ に置く**。
   `TaskId` / `TaskTitle` のような branded primitive は必ず `domain/` に置き、
   `taskId(raw): Result<TaskId, TaskIdError>` 形式の smart constructor を提供する。
   string や number を裸で domain 関数に流さない。

5. **domain 関数は `(state, input) → Result<{ state, events }, Error>` 形を保つ**。
   side effect を返り値で表現すること。例外を使わない (Rust では `Result<T, AppError>`、
   TS では `Result<T, E>` を `shared/types/` から import)。

6. **`shared/contracts/` は空のままで始める**。
   cross-slice 協調が必要になった時に「ここに型を宣言してから両 slice を contract に
   依存させる」流れを尊重する。先回りで埋めない。

7. **テストは slice-local に置く**。
   `slices/<slice-id>/tests/` に置き、同じ slice 内の任意の sub-layer に reach 可能。
   `domain/` の純粋関数を最優先で test し、`application/` は port を fake 化して
   orchestration を test する。

### Never do

1. **cross-slice 直 import**。
   `slices/foo/...` から `slices/bar/...` を `import` / `use` してはならない。
   必要なら contract / event を経由する。

2. **slice 内部に深く reach**。
   slice の外から `slices/<slice-id>/domain/...` を直接 import するのは禁止。
   public entry のみ。

3. **`shared/` から slice を import**。
   shared は dependency graph の最下層。slice を参照してはいけない。

4. **同 layer import**。
   1 つの `ui-widget` が別の `ui-widget` を直接呼ばない。slice の public entry 経由。

5. **Tauri stack の場合**: UI layer から `@tauri-apps/api/core` の `invoke` を直接呼ばない。
   `<slice_root>/shared/ipc/` (tauri-specta-generated bindings)経由のみ。
   ESLint adapter が `forbidden_imports` でブロックする。

6. **bootstrap ファイルを slice に置かない**。
   `package.json` / `tsconfig.json` / `Cargo.toml` 等は upstream framework init の責務。
   ori が生成するのは `.ori/architecture.md` のみ。

7. **`example-slice/` の path をそのまま target に書かない**。
   example-slice は study material であり target にコピーされない。AI が読んで型紙にする
   ためだけのものなので、生成 file は target の `apps/<APP_NAME>/src/<BC_NAME>/...` に書く。

## Test strategy

### Slice 内テストの粒度

| sub-layer | 何を test するか | 何を test しないか |
| --- | --- | --- |
| `domain` | VO smart constructor の境界値、aggregate の state 遷移 / event 発火、不変条件 | I/O、framework 依存 |
| `application` | port を fake/stub 化した状態で、複数 domain 関数の orchestration を verify | DB / network、UI |
| `infrastructure` | 実 adapter の I/O 振る舞い (integration test)、契約 (port interface) を満たすか | domain logic |
| `presentation` | view model 変換、pure render の出力 | DOM / browser (e2e で別途) |

### テスト命名

- `describe("slice:<slice-id> <sub-layer>", () => { describe("<concept>", ...) })` の
  2 段構成 (例参照)。grep でスコープが追えるように。
- `it("rejects <invalid case>", ...)` / `it("accepts <valid case>", ...)` の対称性を保つ。

### 共有 fixture

- 固定時刻 `FIXED_NOW = () => new Date(...)` を slice ごとに用意し、event の occurredAt
  検証を deterministic に。
- UUID 等の sample id は slice 内で 1 つ constant に固める (`SAMPLE_ID = "..."`)。

## Slice Definition of Done (AI への指示)

`pattern.md` の "Slice Definition of Done" section が slice 完成の normative 定義。
AI agent が `/ori-flow` で slice を生成 / 完了判定する際は、以下を強制する:

1. **sub_layers の全埋め**: `domain` / `application` / `infrastructure` /
   `presentation` / `tests` のいずれかが空 or placeholder のままなら slice は未完。
   `tests/` 不在で "後で書く" と先送りしない。

2. **boundary test の経路選択**:
   - 当該 slice の layer が `cross_root` contract に参加している場合
     (例: Tauri stack の rust 側 `domain` slice が tauri-specta の生成側)、
     test は **generator が吐いた binding** 経由で呼ぶ。
     Rust 内部から `application::handle_xxx(...)` を直叩きするのは DoD 違反。
   - `cross_root` がない slice では、test は slice の `<public_entry>`
     (`index.ts` / `mod.rs`)のみ経由で import する。
     `slices/<slice-id>/domain/...` 直 import は不可。

3. **production wiring 強制**: boundary test (rule 2)では production の adapter
   set で slice を構築する。fake/mock を使った orchestration test を DoD として
   数えない。fake test を書きたい場合は `application/` 内部の test module に
   閉じ込め、DoD カウントから除外する。

4. **`cross_root` 同期**: rust 側 source を変えたら specta 等の generator を
   走らせて TS 側 binding を再生成する。`flow-impl-red` / `flow-impl-green`
   の phase hook で rebuild 済か確認 (stack template の指示に従う)。

DoD を満たさない slice は `/ori-review` の test 結果 check で失敗する想定。AI agent
は slice を "完了" と報告する前に、上記 4 点を **生成物単位で** 自己照合すること。

## Migration

### 既存プロジェクトを ddd-vsa-hex に寄せる

1. **BC 抽出を先**: コードを動かす前に DDD 上の BC を 2-3 個 enumerate。strategic design
   結果を `.ori/domain/` 配下に置いて SSoT 化。
2. **slice 抽出は use case 単位**: 「create-order」「cancel-order」「ship-order」のように
   verb-noun で切る。既存 module が機能横断的なら use case 軸に再分割。
3. **shared を最薄化**: 既存の "utils" ディレクトリを `<slice_root>/shared/` に直接
   持ち込まず、必要な branded VO + Result + base event だけに削る。残りは slice 内に
   private に寄せる。
4. **public entry 作成**: 各 slice の `<public_entry>` を作って、外から触られていた API
   を re-export。それ以外を internal にして cross-slice の import を全部この経路に
   集約。
5. **lint enforce**: `.ori/architecture.md` を書き、`node ./scripts/export.js` (ori-arch skill bundle 相対)
   で adapter config を生成 → CI に組み込み、新規違反を block。

### ddd-vsa-hex から離脱する場合

- slice 単位で切り出して別 pattern に移すのは比較的容易 (slice の public entry が
  inter-module 境界と一致しているため)。
- `shared/` の branded VO / Result / event base は普遍的で他 pattern にも持ち越し可能。
- 削るべきは `cross_layer.rules` の declarative 部分 (architecture.md frontmatter 全体)。

## どこに何が定義されているかの早見表

| 探しているもの | 場所 |
| --- | --- |
| layer / slice 全体の依存ルール | `.ori/architecture.md` frontmatter |
| 概念図 / 責務 / 命名規則 | この pattern の `pattern.md` |
| AI 向け do/don't (このファイル) | この pattern の `ai-notes.md` |
| TS 用 worked code | `stacks/typescript/example-slice/` |
| TS + Rust 用 worked code | `stacks/typescript-tauri/example-slice/` |
| `.ori/architecture.md` の元 | `stacks/<stack>/architecture.md.tpl` |
