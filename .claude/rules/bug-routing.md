## バグ報告の検出

ユーザ発話に以下のキーワードが含まれ、かつ `.ori/slices/` ディレクトリが存在する場合、
**即座にコード修正を開始せず**、`/ori-bug` へルーティングすること。

**トリガーキーワード**: `バグ`, `bug`, `fix`, `直ら`, `直して`, `壊れ`, `エラー`, `不具合`, `動かない`, `壊れている`

## 禁止事項（triage 前にやってはいけないこと）

- ❌ ユーザの説明だけで即座に impl を修正する
- ❌ spec.md を guardrail なしで直接編集する
- ❌ ドメインを直さず複数 slice の spec で局所対応する
- ❌ review をスキップする

## ルーティング手順

1. `bd create` で bug issue を起票（type=bug、症状を description に記載）
2. `/ori-bug` skill を起動し、4ケース（domain / impl / spec / cross-slice）の triage を開始する
3. triage 結果に従い、該当する recovery flow を実行する

## 例外

- `.ori/slices/` が存在しないプロジェクトでは通常対応（本 instruction は適用しない）
- ユーザが明示的に「triage 不要、直接修正して」と指示した場合はそれに従う
