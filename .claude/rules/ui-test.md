---
paths:
  - "**/*.{spec,test}.tsx"
  - "**/e2e/**/*.{spec,test}.{ts,tsx}"
  - "**/playwright/**/*.{spec,test}.{ts,tsx}"
  - "**/__tests__/**/*.{spec,test}.{ts,tsx}"
---

# UI テスト規約（ポインタ）

> UI selector / testid 命名の **canonic source** を pattern/stacks 正典へ一本化済み。
> 本ファイルはポインタを持ち、concretion を二重管理しない。

## 正典ポインタ

| 関心事 | 正典 |
| --- | --- |
| 層別 selector 優先順位 (Component: role / E2E: testid) | [`ori-architect/patterns/ddd-vsa-hex/pattern.md`](../../apm_modules/dev-komenzar/ori/.apm/skills/ori-architect/patterns/ddd-vsa-hex/pattern.md) "Test conventions" → "UI selector / testid 規約" |
| testid 命名 (VSA namespace, `.` separator, `<elem>` 機能名) | 同上 |
| page / widget の testid 具体値 (契約) | `.ori/pages/<id>/testids.yaml` — 規範は pattern.md "page / widget の testid 契約" |
| production fixture (`setupProductionBuilder()`) | [`ori-architect/patterns/ddd-vsa-hex/stacks/typescript-tauri/test.md`](../../apm_modules/dev-komenzar/ori/.apm/skills/ori-architect/patterns/ddd-vsa-hex/stacks/typescript-tauri/test.md) "#setup-production-builder" |
| 実装側規約 (Smart Constructor / Result / VSA 配置) | `ddd-typescript.instructions.md` |

## 責務分離

- 本ファイル = UI テスト層に効く rules の **glue**（`applyTo` で TS テストファイルに適用）。
- selector / testid / fixture の規範本文は pattern/stacks 正典が担う。
- domain test のメタルールは `ddd-test.instructions.md`、シナリオ E2E は `scenario-test.instructions.md`。

UI コンポーネントの単体テスト (`*.test.tsx`) は `ddd-test`（メタルール）と本ファイル
（UI 層）が補完的に効く。`ddd-test` は「spec トレース / adapter 境界 mock の心構え」、
本ファイルは「DOM query 時に getByRole / data-testid を VSA 命名で使う」と直交する。

## 既存実装の testid を契約へ移行する {#testid-migration}

契約 (`testids.yaml`) より前に実装された page / widget は、ui-field id (`screen-<N>-*`) などを testid に
使っていることがある。その場合、契約由来の testid を使う生成テスト (scenario / component test) は必ず RED になる。
直すのは実装側で、契約は変えない。実装 testid を契約の alias として登録することもしない
(契約が唯一の正典。ori-oan.13)。

**入口**: `testid-violation` + `page:<id>` の bd issue。issue は `/ori-generate` (実装のある参加 page だけ) と
`/ori-doctor --testid-sweep` (全 page) が `check-page-testids.sh --emit-issues` で起票する。
`/ori-bug` は、この issue を本節へ案内する。移行は page ごとに 1 issue 単位で行う。
以下の `scripts/testids.js` は、移行を実行する skill (`/ori-impl-green` / `/ori-derive` 等) の bundle にある。

1. **経路を決める**。spec.md が正しいかどうかで分ける:
   - spec.md に `screen-<N>-` 形式の testid 記述 (「field id を data-testid に写す」等) がある、または
     root・region・動的要素が `extra:` に登録されていない → `/ori-flow <page-id>` で derive からやり直す
     (spec の欠陥。`/ori-bug` のケース 3 に当たる)。derive が spec を直し、`extra:` を登録する
   - spec.md が正しい → `/ori-impl-green <page-id>` → `/ori-review <page-id>` (実装の欠陥。ケース 2 に当たる)
   - 判定には `testids.js check <page-id> --no-impl` と `grep -n 'screen-[0-9]*-' .ori/pages/<page-id>/spec.md` を使う
2. **置換対応表を出す**: `node scripts/testids.js migrate-map <page-id>`。
   derived 行ごとに「旧 testid (field id) → 契約 testid」と、実装・テストでの使用箇所 (`impl` / `test`) を出す。
   数えるのは testid の値の位置 (`data-testid="…"` / `getByTestId('…')` 等。改行をまたいでもよい) だけ。
   それ以外の箇所は `参考` 行に出る。`id=` / `for=` / `name=` などは testid ではないので置換しない。
   ただし定数経由 (`const TID = { title: "screen-1-title" }` を `data-testid={TID.title}` で使う等) は
   testid なので置換する。参考行は 1 件ずつ読んで判断する
   `手動:` の行 (extra の testid、その page の動的 testid / 形式違反) は旧値を推定できないので、実装を読んで対応を決める
   末尾の `未契約の testid` 節は、その page に帰属する実装 testid のうち契約 (derived / extra) にも旧 field id にも無いもの
   (旧 root の値や `screen-N-*` の残り。位置付き)。契約に足す (`add-extra`) か実装から消すかは人間が決める。
   件数・exit code には含まれないので、移行後に節が空になっているかを目で確認する
3. **機能欠落を切り出す**。`migrate-map` の derived 行で `impl` の使用箇所が 0 件の契約 testid は、
   付け替えではなく実装に UI 要素が無い (機能欠落)。`check` は literal の存在を要求するので、要素を足さない限り exit 0 にならない。
   推測で要素を足さず、次のように別 issue へ切り出す:
   - spec に挙動規定 (不変条件 / test-points) がある → 実装の欠陥。`/ori-impl-green` 向けの bd issue を起票する (ケース 2)
   - spec に挙動規定が無い (ui-fields とレイアウト図にだけ出る等) → spec の欠落。derive からやり直す bd issue を起票する (ケース 3)
   - 移行 issue は、切り出した issue に `bd dep` で blocked-by にする。完了条件は緩めない (check の許容リストは設けない)。
     欠落以外の置換は、blocked のまま続けてよい
4. **実装と unit test を同じ変更で置換する**:
   - 実装は契約の testid を literal で付ける。動的 testid (`` data-testid={`x-${key}`} ``) は、固定 testid + `data-key={key}` に直す
   - app の unit test (component test 等) の selector も契約値に置き換える。動的要素は `[data-testid="<固定 testid>"][data-key="<key>"]` で絞る。
     置き換えるのは selector だけで、assertion は変えない
5. **完了を判定する**。次の 3 つがそろったら issue を close する:
   - `node scripts/testids.js check <page-id>` が exit 0
     (page を指定した check は、その page に帰属する実装違反だけを数える。他 page の移行が済んでいなくても止まらない)
   - `node scripts/testids.js migrate-map <page-id>` が exit 0 (旧 testid の使用が 0 件)
   - app の unit test が通る

   機能欠落の issue が残っている間、移行 issue は close できない (手順 3 の blocker)。

scenario の GREEN は移行の完了条件に含めない (app 側の不具合が混ざるため、別に検証する)。
`page:_impl` の issue (どの page に帰属するか決まらない実装違反) は、各 page の移行が済んだあとに
`testids.js check --all` で残りを確認して直す。帰属の規則を入れる前 (ori-oan.13 以前) に起票された
`page:_impl` issue には、いまは各 page に帰属する違反も入っている。再 sweep で page 別の issue ができたら close する。
