# Review: check-for-updates {#review-check-for-updates}

> ori-flow phase 6 (slice). Base diff: `3dfaa7e..HEAD` (ori-m2ra.4 — I-U3 改訂 + I-U4 新設の再 derive)。
> structural gate (a) tests green / (b) lint / (c) public_entry / DoD は機械実行で PASS 済み。
> 本 pass では **spec ↔ impl / domain の意味的乖離** のみ審査する。

## Pass 1 {#pass-1}

### 1. spec 整合性 {#pass-1-spec-coherence}

- I-U1 (immutable `current_version`), I-U2 (`latest > current` 単一エントリ `compare`), I-U3 (24h 周期 + 揮発 + 即時リトライなし), I-U4 (通知済み以下は非発行 / 揮発 / application 責務) いずれも
  `application.rs` の `try_execute` / `domain.rs::{is_check_due, should_notify, UPDATE_CHECK_INTERVAL=24h, PERIODIC_CHECK_TICK=1h}` / `infrastructure.rs::InMemoryUpdateCheckState` に 1:1 で写像されている。
- C-CFU1 (`Result` 露出なし) は `execute` / `check_for_updates` command / `UpdateChannelResponse` のすべてで保持。TP-S14-3 / TP-T1 の fn pointer coercion で compile-time pin も掛かっている。
- C-CFU8 (判定と記録を 1 操作) は `InMemoryUpdateCheckState::claim` が単一 `Mutex<Option<Version>>` を取り、`should_notify` 判定 → `*notified = Some(...)` を同一ロック下で行う実装になっており、二重通知の race を構造的に排除している。
- C-CFU10 (開始時に基準更新・成否不問) は `check_for_updates` command が `begin_check(SystemTime::now())` を先に行い、その後で `execute_update_check` を呼ぶ順序で担保。
- C-CFU11 (wall clock + 1h tick + 巻き戻り即 due) は `is_check_due` が `SystemTime::duration_since` の `Err` (= 巻き戻り) を `true` として扱い、`spawn_periodic_update_check` が `tokio::time::sleep(PERIODIC_CHECK_TICK)` 後に `SystemTime::now()` で判定するため、tokio timer が suspend で止まっても復帰直後の tick で due になる。
- C-CFU12 (起動時 invoke と定期 task が同じ揮発 state を共有) は `lib.rs` が `Arc<InMemoryUpdateCheckState>` を `.manage` し、`check_for_updates` command と `spawn_periodic_update_check` の両方が `app_handle.state::<Arc<InMemoryUpdateCheckState>>()` 経由で取得しているため配線通り。

### 2. derives_from 網羅 {#pass-1-derives-from}

- `workflows/check-for-updates.md#check-for-updates` → pipeline 7 ステップ (fetch → parse → compare → branch → build → suppressNotified → emit) が `try_execute` + `claim` の順序に正確に対応。
- `aggregates.md#update-channel-aggregate` の I-U1〜I-U4 は spec #invariants-update-channel-aggregate に全条引用され、テスト TP-I1 / TP-I2 / TP-I3 / TP-D* / TP-P* で検証されている。
- `domain-events.md#new-version-detected` の payload (`current_version` / `latest_version` / `release_url` / `release_notes`) は Rust `NewVersionDetected` と `NewVersionDetectedPayload`、TS 側 payload 型すべてで一致 (TP-N3 で確認)。
- `validation.md#s14-update-check-failure` / `#s23-update-same-version-once` は TP-S14-* / TP-D1〜D7 に写像され、特に S23 の「When 1: event 非発行だが戻り値は Some」が TP-D1 で明示 assert されている。
- `bounded-contexts.md#update-distribution` の「確認する / 通知する / 現在のバージョンを提示する」の責務境界は port 3 本 (`UpdaterPort` / `EventBus` / `NotifiedVersionLedger`) として明示化されており、generic subdomain 性 (Tauri plugin へ外注) も `TauriUpdaterPort` に閉じている。

### 3. DDD 規約遵守 {#pass-1-ddd-compliance}

- `domain.rs` は I/O ゼロ (pure const + pure fn のみ)、`application.rs` も port 越しのみ副作用。`try_execute` が `Result` を使うが `execute` が握り潰すため boundary 露出は無い (C-CFU3)。
- `NotifiedVersionLedger` trait は port として `shared/ports.rs` に置かれ、production 実装 `InMemoryUpdateCheckState` は `infrastructure.rs` 配下。aggregate が 1 回分の確認結果を返すだけで、通知済み state を保持しない原則 (I-U4 後段) を守っている。
- `shared/ports.rs` の `Rc<T>` / `Arc<T>` blanket impl により、production (`Arc`) と test (`Rc`) で同じ型を共有できており、production fixture (`setupProductionBuilder` 相当) の実物流用 (DoD rule 3) が成立している。

### 4. 副作用の境界 {#pass-1-side-effects}

- Network I/O (`updater.check()`): `TauriUpdaterPort::fetch_latest_release` のみ。
- Event publish: `TauriEventBus::publish` のみ。
- 揮発 state 書換: `InMemoryUpdateCheckState` の `Mutex` 2 本のみ。
- `check_for_updates` async command 内で `execute_update_check` (sync) が `block_in_place` 経由で `updater.check().await` を呼び出すが、Tauri の async runtime は multi-thread のため `block_in_place` の前提は満たしている (infrastructure.rs のコメントで宣言済み)。

### 5. edge case / テスト妥当性 {#pass-1-edge-cases}

- TP-S14-4b (`Ok` で返った `"0.4.0-rc1"` が parse 不能) は Ok path の silent fallback をちゃんと塞いでいる。pre-release reject の暫定方針が regress した場合もここで検知可能。
- TP-D3 (`"0.2.4"` 通知済み → `"0.2.3"` 受信) は I-U4 の「以下」(not just equal) を明示 assert。言語の曖昧さを埋める good edge。
- TP-D5 (失敗 / UpToDate / Older で ledger 不変) は C-CFU9 を外形で検証。
- TP-D6 (proptest) は「公開バージョン列は狭義単調増加」+「current より大きい最大値は必ず最後に publish」を 2 面から縛っており、I-U4 + I-U2 の合成不変を強く保証している。
- TP-P2 (24h-1s / 24h / 24h+1s)、TP-P3 (巻き戻り)、TP-P7 (proptest) で境界と全域が揃っている。
- TP-D7 / TP-D8 で `should_notify` 純粋関数と `claim` の atomic 挙動を単独で pin している。

### 6. テスト ↔ spec トレース {#pass-1-trace}

- `tests.rs` の各 `#[test]` doc comment / assertion message に `spec.md#tp-*` の anchor が明記されており、相互 grep 可能。describe/outer ラッピングは Rust 慣例に従い section コメント `// ===== TP-N*: ... =====` で代替。十分。

### 7. 冗長性 / premature opt {#pass-1-redundancy}

- `shared/ports.rs` の `Rc` / `Arc` blanket impl は既存流儀。premature ではなく、test rig と production wiring の両立に必要。
- `rig` / `rig_seq` / `new_state` / `ok(version)` ヘルパは乱雑でなく、 D* シリーズの rig_seq pattern は 1 試験 1 state の流れが読みやすい。

### 検出事項 {#pass-1-findings}

#### F1 (LOW): `spawn_periodic_update_check` が tick ごとに `state` を re-lookup する {#f1}

- 位置: `commands.rs::spawn_periodic_update_check` ループ内 `app_handle.state::<Arc<InMemoryUpdateCheckState>>()`。
- 内容: `AppHandle::state()` の lookup は hash lookup を伴うだけで実害は無いが、`.manage` は 1 度しか呼ばれないため、ループ外で 1 回取得して `Arc::clone` を回す形に出来る。純粋な cosmetic。
- 推奨: ループ前に `let state = app_handle.state::<Arc<InMemoryUpdateCheckState>>().inner().clone();` を外に出す (任意)。severity LOW。

#### F2 (LOW): `check_for_updates` command の暗黙前提 (multi-thread runtime) に module doc がない {#f2}

- 位置: `commands.rs::check_for_updates` / `execute_update_check`。
- 内容: `execute_update_check` は sync 関数だが内部で `block_in_place` を伴う (`TauriUpdaterPort::fetch_latest_release`)。infrastructure.rs 側にはコメントがあるが、`commands.rs` を単独で読むとわからない。将来 `tauri::async_runtime` を単一 thread に差し替えると panic する暗黙前提。
- 推奨: `execute_update_check` doc comment に「tauri::async_runtime の multi-thread 前提 (block_in_place を内包)」の 1 行注記を足す。severity LOW。

#### F3 (LOW): frontend 再 invoke で前回確認時刻が張り直される挙動 {#f3}

- 位置: `commands.rs::check_for_updates` (`state.begin_check(SystemTime::now())` 無条件)。
- 内容: dev 環境で page-main が hot reload により複数回 mount されると、そのたびに `begin_check` が呼ばれ 24h 基準が張り直される。production では page-main の listen 確立は 1 回のみなので実害なし。spec #impl-periodic の「起動時 1 回」契約に frontend 側が従う前提に乗っている。
- 推奨: `commands.rs` に 「page-main の mount-time effect に対する 1 回切り契約」を参照する注記を追加 (任意)。severity LOW。

#### F4 (LOW): I-U4 揮発性に関する ledger の生存範囲が wiring 側のドキュメントに暗黙的 {#f4}

- 位置: `lib.rs` の `.manage(Arc::new(InMemoryUpdateCheckState::new(SystemTime::now())))`。
- 内容: Tauri `State` は AppHandle 生存期間 = プロセス生存期間に紐づくため I-U4 の「揮発 (再起動後は再び通知)」は成立するが、`lib.rs` のコメントは I-U3 / I-U4 の両方を括って説明しているだけで、再起動 = 新 state という domain 条件への対応が明示されていない。TP-D4 で test 側は検証済み。
- 推奨: 既存コメントに「再起動 = 新 process = 新 `InMemoryUpdateCheckState` ⇒ I-U4 揮発を満たす」の 1 行を追加 (任意)。severity LOW。

### 総合判定 {#pass-1-verdict}

I-U3 / I-U4 の domain 改訂が spec に正しく派生され、実装 (純粋関数 / 揮発 state / 定期 task / Tauri command / frontend 配線) にも破綻なく下りている。テストは `tp-periodic` / `tp-notified-once` 両系統を境界値 + proptest + ledger の実物で押さえており、未検証な分岐は無い。副作用境界と `Result` 露出禁止 (C-CFU1) もコード・テスト・型レベルで三重に pin されている。
検出した 4 件はいずれも cosmetic / doc 補強で、spec・domain 文書に根拠を持つ乖離や欠陥ではない。

verdict: PASS
