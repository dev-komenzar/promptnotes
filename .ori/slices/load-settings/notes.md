# load-settings — Implementation notes

## 2026-06-25 — deferred findings triage (ori-9i0)

review.md Pass 1 で deferred とした MED/LOW findings を triage し、以下の方針で対応した:

### Domain 直接編集

- `domain/workflows/load-settings.md#notes` を更新 (Q5 + Q6 (a) + Q7):
  - field-level fallback を「規定」として格上げ
  - JSON が Object でない場合は「全フィールド欠損 Object」として field-level 経路を通る (degenerate case) と明示
  - 冪等性は `FileSystem::ensure_dir` (mkdir -p 相当) の責務と明記
- `domain/workflows/load-settings.md#steps` の step 2-4 を Object-based pipeline に更新
- `domain/aggregates.md#settings-aggregate-invariants` I-S2 を更新 (Q4):
  - 判定方向を明文化 (`config_path.starts_with(storage_dir)`)
  - **port-level 契約** として「OS 慣習パスを返す port は I-S2 を保証する」を追記

### spec.md の再整理

`/ori-sync` が MVP stub のため `spec.md` は手動で更新 (frontmatter hash も再計算):

- `#invariants-settings-aggregate` I-S2 文言を port-level 契約反映に更新
- `#invariants-slice-specific` C-LS3 / C-LS4 / C-LS8 を field-level fallback + FS impl 責務に統合
- `#tp-partial` TP-PT3, `#tp-invariants` TP-I2 / TP-I4 を新規定にあわせて書き換え
- `#impl-pipeline` / `#impl-serde` を `Value` ベースに書き換え (`SettingsRaw` DTO は使わない)
- `#impl-deps` に I-S2 port 契約と冪等性責務を明記
- `#open-questions` oq-field-level-fallback / oq-no-result-typelevel を RESOLVED に更新

### コード変更

- `ports.rs`: `OsDirs::default_storage_dir` の doc comment に I-S1 + I-S2 契約を明記 (Q4)
- `application.rs`: module doc comment で「pure: no I/O, side effects via ports」を明示 (Q9)

### 動作確認

- `cargo test --lib` → **52 passed, 0 failed**
- `cargo clippy --all-targets -- -D warnings` → clean
- behavior は変更なし (spec / domain の文言を impl と consistent にしただけ)

### create-note 側 (ori-9am)

すべて won't fix で close。理由は read-note slice 着手時に YAML parser / frontmatter 仕様と合わせて再評価するため。詳細は ori-9am の close 理由を参照。
