# check-for-updates — Implementation notes

## release infra blocker (ori-6l4 依存)

本 slice の scope は **domain types + use case + UpdaterPort 境界 + FakeUpdater (test)** に限定。production wiring は **`ori-6l4` (Wire tauri-plugin-updater after release infra is ready) で blocked**。

ori-6l4 完了後の別 follow-up issue (ori-2lm.8) で以下を追加する:

- `update_distribution/slices/check_for_updates/infrastructure.rs` に `TauriUpdaterPort` 実装
- `apps/promptnotes/src-tauri/Cargo.toml` の `tauri-plugin-updater` 依存（既に追加済だが現状は idle）の features を有効化
- `apps/promptnotes/src-tauri/tauri.conf.json` の `plugins.updater.endpoints` / `pubkey` 設定
- GitHub Releases 署名鍵の運用フロー（CI / 開発者ローカル）
- `lib.rs` の `invoke_handler!` に `check_for_updates` command を追加（commands.rs 新設）

本 slice 単体では production からは **呼び出されない**（無害な domain code として存在）。

## domain aggregates.md の `check_at_startup` signature

`aggregates.md#update-channel-aggregate-operations` の `check_at_startup() -> Result<UpdateChannel, UpdateError>` は spec C-CFU1（no-Result outer API）と矛盾する。slice 側は spec を信じて `execute -> UpdateChannel` を実装したが、domain doc 側で「outer service が握り潰す」旨を明示しないと next slice / ori-6l4 wiring 時に再ブレる。

→ proposal `.ori/proposals/pending/2026-06-26-check-for-updates-aggregates-check-at-startup-no-result.md` で upstream 修正を提案。follow-up issue: ori-2lm.11。

## Version VO の scope

- 本 slice では `major.minor.patch` の 3-tuple lexicographic 比較のみ実装（spec.md#oq-version-pre-release）
- pre-release / build metadata を含む文字列は `ParseError` で reject（保守的 fail-silent）
- GitHub Releases の tag 形式は ori-6l4 wiring 時に確認 → 必要なら follow-up issue (ori-2lm.9) で `Version` を strict semver 対応に拡張

## review 指摘の対応 (phase 6 → phase 7 へ送られた fix)

- (6.a) TP-S14-4b: Ok 返却で version_string が parse 不能な path のテスト追加（`0.4.0-rc1` ケース）
- (6.b) TP-R1b: UpToDate / OlderVersion path でも UpdaterPort 呼出が 1 回であることを assert（C-CFU4 全 path 網羅）
- (7.a / 本ファイル) blocker と follow-up issue を notes.md に固定
- (1.a) aggregates.md の signature 修正 proposal 起票
