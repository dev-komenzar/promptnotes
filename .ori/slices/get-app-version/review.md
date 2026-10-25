# Review: get-app-version {#review-get-app-version}

## Pass 1 {#pass-1}

### Structural gates

- **Rust tests (`cargo test`)**: PASS (12/12, given)
- **TS tests (`vitest`)**: PASS (1/1, given)
- **Lint / format (eslint + prettier + clippy/fmt)**: PASS (given)
- **public_entry 遵守** (tests が `application` / `domain` を直接通過して command 境界を迂回していないか): PASS (given。sibling `super::domain::to_app_version` は domain 純粋関数テスト、`super::get_app_version` は public_entry 経由。両者が spec#tp-* に対応)

### Semantic findings {#semantic-findings}

spec.md と domain docs を通読し、意味的乖離・観点漏れ・解釈ずれを探索した結果、HIGH / MEDIUM の指摘なし。LOW のみ以下に記す。

- **LOW** `apps/promptnotes/src-tauri/src/update_distribution/slices/get_app_version/tests.rs:tp_p2_unparseable_falls_back_to_raw` — proptest 戦略が `".*"` + `prop_assume!(Version::from_str(&s).is_err())` 構成。`.*` が偶発的に `"a.b.c"` 形の数値 3 連 (parse 可能 semver) を生成した場合に prop_assume による捨てが発生する。実際には衝突確率は極小で runtime 影響なし。/ 推奨: 現状のまま許容。将来 flaky な `too many rejections` エラーが出た場合のみ戦略を `"[^0-9.].*"` 等に絞る。差し戻し不要。

- **LOW** `.ori/slices/get-app-version/spec.md#tp-boundary` + `tests.rs:tp_b1_command_returns_cargo_pkg_version` — TP-B1 の assertion `get_app_version() == env!("CARGO_PKG_VERSION")` は、現 Cargo.toml version (`0.2.2`) が parse 可能 semver であることに依存する。pre-release リリース (`0.3.0-rc.1`) に版が切り替わると正規化後も raw と一致 (C-GAV3 fallback) のため assertion は依然 pass するが、`a.b.c` 以外の leading-zero 入り (`01.02.03` → `1.2.3`) に変わった場合は失敗する。/ 推奨: spec.md#impl-notes にも「正規化後も同値」の前提が明記されており、Cargo.toml に leading zero は入らない運用のため現状許容。差し戻し不要。

- **LOW** `apps/promptnotes/src-tauri/src/update_distribution/slices/get_app_version/application.rs` — `GetAppVersionUseCase::execute` が `_query: GetAppVersionQuery` を受け取るが、入力なし query のためこの引数は常に unused。spec.md#io-input が `struct GetAppVersionQuery;` を定義しているので型自体の存在は正当だが、use case が受け取る必要性は domain 的に薄い。/ 推奨: 現状のまま許容 (DMMF workflow pipeline の pedagogical consistency のため)。差し戻し不要。

### 観点網羅性チェック {#coverage}

- **spec#invariants**: C-GAV1 (fn()->String) → TP-T1 ✓ / C-GAV2 → TP-N1,N2,P1 ✓ / C-GAV3 → TP-F1,F2,F3,P2 ✓ / C-GAV4 (副作用なし) → コードレベルで `log::warn!` 等を含まない ✓ (check_for_updates は log::warn するが本 slice は意図的に silent、spec.md#impl-notes「log を出さない」と一致) / C-GAV5 (情報源一致) → TP-B2 + `shared/build_info.rs` + check_for_updates/commands.rs が `build_version()` に書き換わっている ✓ / C-GAV6 (冪等) → TP-P3 ✓
- **spec#io**: 入力 `GetAppVersionQuery` ✓、出力 `String` (Result で包まない) ✓、domain event なし ✓、Errors なし ✓
- **spec#boundary-contract**: `#[tauri::command] pub fn get_app_version() -> String` ✓、`mod.rs` の `pub use commands::get_app_version` ✓、`lib.rs` の `invoke_handler!` に 1 行追加 ✓、TS `getAppVersion(): Promise<string>` ✓
- **spec#impl-layers**: `mod.rs / domain.rs / application.rs / commands.rs / tests.rs`、infrastructure なし ✓、ビルド時定数は `shared/build_info.rs` に切り出し check_for_updates と共有 ✓
- **derives_from**: workflow / aggregate / BC いずれも参照済み、I-U1 (immutable 定数) に沿う挙動、I-U2/I-U3 への影響なし ✓

### Verdict {#verdict}

**PASS**

理由:
1. spec.md の invariants (C-GAV1〜C-GAV6) すべてがコードとテストに反映されている
2. derives_from の domain section (workflow / aggregate / BC) が manifest 宣言通りに実装へ写像されている
3. test set が TP-N / TP-F / TP-P / TP-B / TP-T のすべての spec#test-points を網羅する
4. 境界契約 (public_entry、invoke_handler 配線、情報源共有 `build_version()`) が spec 通りに実装されている
5. 検出した 3 件はいずれも LOW (運用前提に依存する軽微な注意点のみで、spec / domain 文書に根拠を持つ乖離は無い)

## Pass 2 (re-derive 2026-10-25) {#pass-2}

> ori-m2ra.4 (I-U3 改訂 + I-U4 新設) に伴う再 derive。本 slice は文言追随のみで実装・テスト共に変更なし。

### 1. 変更差分の確認 {#pass-2-diff}

- spec.md: `last_derived` と domain hash を更新。`invariants-update-channel-aggregate` section に I-U4 を「本 slice の対象外」として追記し、`workflows/get-app-version.md#notes` の引用に I-U3 の表現を「起動時 + 24 時間ごと」へ揃えた。
- `commands.rs` / `domain.rs` / `application.rs` / `tests.rs` に変更なし (git diff で確認)。
- `AppVersion` / `to_app_version` / `GetAppVersionUseCase` / `build_version()` 共有の情報源 (C-GAV5) は Pass 1 時点のまま。

### 2. domain 再写像の整合性 {#pass-2-coherence}

- I-U1 (immutable) のみを依存する slice であり、I-U2 / I-U3 / I-U4 は spec #invariants-update-channel-aggregate で「対象外」と明示されている。
- `workflows/get-app-version.md#notes` の引用も改訂後の表現 (「起動時 + 24 時間ごと」) に揃っており、`check-for-updates` の I-U3 改訂と文言ずれなし。
- `aggregates.md` の hash (`e3aba26a3001`) は `check-for-updates` 側の記録と一致 — I-U4 を含む版が参照されている。

### 3. 検出事項 {#pass-2-findings}

- 新規検出なし。Pass 1 の findings も依然 LOW のままで実装への影響は無い。

### 総合判定 {#pass-2-verdict}

本 slice は domain hash 追随と「対象外」注記の文言追加のみで、`get-app-version` 本体の振る舞い (C-GAV1〜C-GAV6) は spec・impl・test 共に Pass 1 PASS のまま維持されている。I-U4 新設は本 slice の責務外であることも明示されており、意味的乖離は無い。

verdict: PASS
