---
ori:
  schema:
    propagation_level: file
  type: review
review:
  slice_id: remove-tag
  reviewer: ori-review (phase 6, fresh-context adversarial)
  date: 2026-06-26
---

# remove-tag review {#remove-tag-review}

## Pass 1 {#pass-1}

### 観点 1: spec 整合性 {#findings-1-spec-coherence}

**[1-HIGH-A] `RemoveTagDiff` variant が domain workflow step 2 と矛盾** — `apps/promptnotes/src-tauri/src/note_capture/slices/remove_tag/application.rs:26-29`

- domain/workflows/remove-tag.md#steps step 2 は `TagDiff = Unchanged | Removed(Tag)` と payload 付きで宣言している
- impl は `RemoveTagDiff { Unchanged, Removed }` で payload を落としている
- spec.md#impl-related-slices も「`RemoveTagDiff = Unchanged | Removed` を application 層で定義」と書いており spec 自身が upstream workflow と矛盾している
- 現状の `Note::remove_tag(&str, now)` 経由では実用上 Tag instance を運ぶ必要がないが、(a) 上流 doc との signature 一致、(b) 将来 audit log で「実際に消した Tag」を識別したい場合の拡張性、(c) compute_diff の戻り値だけで「どれを消したか」を log/event/UI 通知できる、という 3 点で `Removed(Tag)` の方が情報量が多い
- **推奨対応**: 以下のいずれか
  - (1) impl を `Removed(Tag)` に変更し、`apply_remove` で `tag.name()` を渡す（upstream に揃える、minimal patch）
  - (2) workflow doc 側を `Removed` に proposal し down-grade（情報量を捨てる選択を upstream に反映）。`/ori-propose` で domain/workflows/remove-tag.md#steps への変更を提案

**[1-MED-B] spec の derives_from に `domain/workflows/remove-tag.md#errors` が暗黙不一致のまま** — `.ori/slices/remove-tag/spec.md:56-64`

- upstream `domain/workflows/remove-tag.md#errors` は 2 variant (`NoteNotFound` / `PersistError`)
- spec / impl は 3 variant (`LoadError` 追加)。spec.md#io-errors と I-RT8 で理由を明文化している点は良い
- ただし assign-tag spec が `#open-questions` セクションで OQ を残し proposal の余地を track しているのに対し、本 slice の spec には Open Questions セクション自体が無い。LoadError 追加は明示的な「upstream 不採用 / 局所拡張」決定だが、その記録手段（OQ + 後追い proposal）が欠落
- **推奨対応**: spec.md に `## Open Questions {#open-questions}` を追加し、最低 2 つの OQ を記録:
  1. `#oq-error-variant-divergence` — workflow#errors と impl の 3 variant 構成の乖離。assign-tag 同型踏襲の根拠と、上流 proposal の要否
  2. `#oq-remove-tag-now-injection` — `Note::remove_tag(self, tag_name, now)` が aggregates.md#note-aggregate-commands の `Note::remove_tag(self, tag_name: &str) -> Note`（now 無し）と signature 不一致。assign-tag OQ `#oq-assign-tag-now-injection` の同型問題

### 観点 2: derives_from 網羅 {#findings-2-derives-from}

**[2-PASS]** 4 つの upstream section (`workflows/remove-tag.md#remove-tag` / `aggregates.md#note-aggregate` / `bounded-contexts.md#note-capture` / `domain-events.md#note-tags-changed`) はすべて spec 本文・I-RT* で参照され、impl 上の振る舞いに反映されている。

- workflow steps 1-5: application.rs の 6 step pipeline にマップ済 (step 2-3 を `compute_diff` + `match` に分解)
- aggregates.md の I-N1/I-N3/I-N5/I-N6: TP-IM1 / TP-H1 (updated_at) / TP-AI1 でカバー
- bounded-contexts.md#note-capture: BC = note_capture モジュールに配置済
- domain-events.md#note-tags-changed-payload: TP-EP1 / TP-LT1 で TagSet 全体運搬を検証

**[2-LOW-C] aggregates.md#note-aggregate-commands と impl の signature 不一致が記録されない** — `.ori/domain/aggregates.md:81-82` vs `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs:84`

- aggregates.md: `Note::remove_tag(self, tag_name: &str) -> Note`
- impl: `pub fn remove_tag(self, tag_name: &str, now: Timestamp) -> Self`
- 1-MED-B の OQ #oq-remove-tag-now-injection で record すべき
- **推奨対応**: 1-MED-B と統合

### 観点 3: DDD 規約遵守 {#findings-3-ddd}

**[3-PASS]** pure code と I/O の分離は適切。

- `domain.rs` — value type のみ (Command / Error)、I/O なし
- `application.rs` — port 経由でのみ I/O 接触、`Result` 型で表現 (throw 無し、Rust なので panic も無し)
- `commands.rs` — Tauri 層に限定して `tauri::*` を import、他層は依存しない
- Note aggregate (`note.rs`) は pure (Clock を保持しない、`now: Timestamp` を引数で受ける)

**[3-MED-D] `Note::remove_tag` aggregate signature の type weakness** — `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs:84`

- `assign_tag(self, tag: Tag, now: Timestamp)` は validated value object を受け取るのに対し、`remove_tag(self, tag_name: &str, now: Timestamp)` は raw `&str` を受け取る
- I-RT1 が「UI 契約信頼の pragmatic choice」として `&str` を許容する根拠を提供しているが、それは **use case 層** での話。aggregate API の signature まで弱型にする必要はない
- 対称的な選択肢:
  - `Note::remove_tag(self, name: &TagName, now)` — `TagName` newtype を導入し正規化済みであることを型で保証
  - `Note::remove_tag_by_name(self, name: &str, now)` — 命名で「&str を受ける異質な API」を明示
- aggregate level での I-N6（正規化規則）保護は構築時に Tag が保証しているので、`&str` を取る現 API は I-N6 不変条件を間接的に gateway する責任を呼び出し側に押しつけている
- **推奨対応**: 当面は許容（5 ファイル変更で済む scope ではない）。OQ として記録し、`TagName` newtype 導入 proposal を後続 slice (`apply-tag-filter` 等) の検討に回す

### 観点 4: 副作用の境界 {#findings-4-side-effects}

**[4-PASS]** 副作用順序の不変条件 (I-RT3 / I-RT4 / I-RT5) が application.rs の pipeline で正しく実現されている。

- step 5 `write` 失敗時の `?` 早期 return で step 6 `publish` がスキップされる (I-RT4) → TP-PE1 で検証済
- step 1 `load_by_id` 失敗時に step 5 / 6 がスキップされる (I-RT5) → TP-NF1 / TP-LE1 で検証済
- `clock.now()` 呼び出しは diff check の **後** に置かれている (application.rs:72) → no-op path で clock が tick しない、テスト時の Fixed Clock 期待値とずれない

**[4-LOW-E] aggregate 層の no-op 防御が実用上 dead code** — `apps/promptnotes/src-tauri/src/note_capture/shared/types/note.rs:84-88`

- use case 側で `compute_diff` を呼んで `Unchanged` の場合は早期 return するため、`Note::remove_tag` が呼ばれる時点で「該当 tag は必ず存在する」ことが保証される
- それでも aggregate 側に no-op 分岐が残されている。spec.md#impl-aggregate-extension は「aggregate 自体は防御的に no-op するが、event-emission control は use case 責務」と意図的にこの redundancy を採用している
- 妥当な defensive programming であり許容範囲。ただし「use case を bypass して aggregate を直接呼ぶ呼び出し元」が現状存在しないため、real-world での価値は限定的
- **推奨対応**: 現状維持。今後 aggregate を library として外部公開する場合に意味を持つ

### 観点 5: edge case / テスト網羅性 {#findings-5-test-coverage}

**[5-MED-F] `tag_name = ""` (空文字) の挙動が pin されていない** — `apps/promptnotes/src-tauri/src/note_capture/slices/remove_tag/tests.rs`

- spec.md I-RT1: 「空文字 / 禁止文字を含む文字列が来た場合は『存在しない』として no-op になる」
- 既存 TP-NU1 は `" RUST "` (whitespace + case) を pin
- 既存 TP-NM1 は `"python"` (存在しないタグ名) を pin
- しかし `""` (pure empty) のテストが無い。`""` は `Tag::new("")` が `TagError::Empty` で reject するため、TagSet 内に存在し得ない値であり、no-op になることが I-RT1 から導出される
- assign-tag が UI 契約違反時の安全側挙動の test を網羅していることと対称性が欠ける
- **推奨対応**: TP-NU2 を追加:
  ```rust
  #[test]
  fn tp_nu2_empty_tag_name_yields_noop() {
      // I-RT1: 空文字も「存在しない」として no-op になる
      ...tag_name: "".to_string()...
      assert!(result.is_none());
  }
  ```

**[5-LOW-G] no-op path での Note 永続化状態の immutability 未検証** — `apps/promptnotes/src-tauri/src/note_capture/slices/remove_tag/tests.rs:521`

- TP-IM1 は happy path で「**書き込まれた** Note の id/body/created_at が seed と一致」を検証
- 一方 no-op path (TP-NM1 / TP-NE1 / TP-NU1 / TP-NC1) では「write が呼ばれない」だけを検証し、repo 内の seed Note が触られていないことは未検証
- 現 FakeRepo impl では write を経由しない限り `notes` map は変わらない構造なので実害は無いが、I-RT2 (no-op semantics) を「load-only path も Note を一切変更しない」まで含めて pin したいなら追加テストが欲しい
- **推奨対応**: 優先度低。気になるなら以下を追加:
  ```rust
  #[test]
  fn tp_im2_noop_does_not_mutate_stored_note() {
      // seed → no-op execute → repo.notes[id] が seed と byte-for-byte 一致
  }
  ```

**[5-LOW-H] `parse_note_id` の sentinel epoch fallback が untested** — `apps/promptnotes/src-tauri/src/note_capture/slices/remove_tag/commands.rs:108-115`

- 不正な note_id (例: `"not-a-timestamp"`) が UNIX_EPOCH に降格 → 結果として `NoteNotFound` となる
- commands.rs は Tauri 層なのでテスト困難だが、`parse_note_id` 自体は pure helper として unit test 可能
- assign-tag も同じ問題を抱えている (spec.md#oq-invalid-note-id-reuse) ため、本 slice 固有の問題ではない
- **推奨対応**: 上流 OQ (assign-tag #oq-invalid-note-id-reuse) を本 slice の OQ にも cross-reference するに留める

### 観点 6: テスト ↔ spec トレース {#findings-6-test-trace}

**[6-PASS]** 各 `#[test]` に `/// spec.md#tp-* TP-*` の doc comment が付与され、トレーサビリティが確立されている。

- 16 test 全てに section anchor reference あり (例: `spec.md#tp-happy TP-H1`)
- TP-EP1 → I-RT7 / TP-PE1 → I-RT4 / TP-SO1 → I-RT3 と invariant ID も assertion message に embed
- TP-SIG (signature pin) も spec.md#io-output を参照

**[6-LOW-I] TP-NU* (no event for unchanged) のセクション ID と TP-NU* (no-op unnormalized) のセクション ID が衝突気味** — `.ori/slices/remove-tag/spec.md:114, 146`

- spec section ids: `#tp-noop-unnormalized` と `#tp-no-event-unchanged`
- test ids: `tp_nu1_whitespace_and_case_diff_yields_noop` と `tp_eu1_unchanged_path_publishes_no_event`
- 一見 spec section `#tp-noop-unnormalized` と test `tp_nu1_*` は対応するが、spec section `#tp-no-event-unchanged` と対応する test prefix が `tp_eu1` で 命名が不揃い (TP-NU* と書きそうな所で TP-EU* になっている)
- spec.md コメント (`// ===== TP-NU*: no event for unchanged =====`) が test prefix `tp_eu1_` と矛盾 (tests.rs:543)
- **推奨対応**: tests.rs:543 のコメントを `// ===== TP-EU*: no event for unchanged =====` に修正、または test を `tp_nu2_*` にリネーム（後者は前述 5-MED-F と衝突するので前者推奨）

### 観点 7: 冗長性 / over-engineering {#findings-7-redundancy}

**[7-LOW-J] `SystemClock` / `NoOpBus` / `parse_note_id` / `resolve_storage_dir` が 3 つの commands.rs 間で重複** — `apps/promptnotes/src-tauri/src/note_capture/slices/{assign_tag,remove_tag,auto_save_note}/commands.rs`

- 3 slice 全てが同一の `struct SystemClock` / `struct NoOpBus` / `fn parse_note_id` / `fn resolve_storage_dir` を local 定義している
- VSA (Vertical Slice) 哲学で「slice 間の重複を許容して結合度を下げる」設計は理解できるが、今後 `NoOpBus` が `RealEventBus` に置き換わる際に 3 + N ファイルを同時 patch する必要がある
- **推奨対応**: 本 slice のレビュー scope 外（先行 2 slice の問題）。次の `ori-refactor` phase で `note_capture/shared/runtime.rs` に促成 promote を検討

**[7-LOW-K] `compute_diff` の重複 scan** — `apps/promptnotes/src-tauri/src/note_capture/slices/remove_tag/application.rs:31` と `note.rs:85`

- `compute_diff` と `Note::remove_tag` の両方で `tags.iter().any(|t| t.name() == tag_name)` を実行
- 4-LOW-E と同根。実用上は O(n) を 2 回回るだけ (TagSet は通常小さい) で性能影響なし
- 明確に「use case が emit を制御」と「aggregate が invariant を守る」の責務分離なので、redundancy は意図的
- **推奨対応**: 現状維持

## Disposition {#disposition}

**判定: NEEDS_FIX** (HIGH 指摘 1 件、MED 指摘 3 件)

### 必須修正 (NEEDS_FIX trigger) {#must-fix}

- **[1-HIGH-A]** `RemoveTagDiff` payload 欠落: impl と spec.md#impl-related-slices が upstream workflow#steps と矛盾している。spec が upstream 文書と矛盾するのは coherence の根本問題。修正方針 (impl を `Removed(Tag)` に揃える / workflow を `Removed` に proposal で down-grade) を選択して反映する必要がある

### 強く推奨 (next /ori-flow phase で対応すべき) {#should-fix}

- **[1-MED-B]** spec.md に `## Open Questions` セクションを追加し、(i) 3 variant error 構成の局所拡張、(ii) `Note::remove_tag(now)` aggregate signature 不一致 を track
- **[3-MED-D]** `Note::remove_tag` の `&str` 受け取りは aggregate type weakness。OQ として記録し、`TagName` newtype 提案を後続検討
- **[5-MED-F]** TP-NU2 (空文字 `tag_name`) を追加

### 任意改善 (LOW) {#nice-to-have}

- [2-LOW-C] / [4-LOW-E] / [5-LOW-G] / [5-LOW-H] / [6-LOW-I] / [7-LOW-J] / [7-LOW-K] は本 slice の品質を下げない。次の slice や refactor phase で順次対応可

### 良い点 {#highlights}

- 16 test 全てが spec section anchor を doc comment で参照しトレーサビリティが完璧
- pipeline step に対応する inline コメント (`// Step 1 — load_note ...`) が impl 可読性を引き上げている
- I-RT4 (write 失敗 → publish 抑制) と I-RT5 (load 失敗 → 副作用ゼロ) を OrderLog + spy で機械的に検証
- assign-tag / auto-save-note との対称性が高く、Note Capture BC の write 系 slice の implementation pattern が確立されている
- aggregate と use case の役割分担 (defensive vs orchestration) が明文化されている

## Pass 2 {#pass-2}

date: 2026-06-26
reviewer: ori-review (phase 6 Pass 2, fresh-context adversarial)
scope: Pass 1 findings 解消の検証 + regression 検出

### 観点 1: Pass 1 findings 解消の検証 {#pass2-findings-1-fix-verification}

**[P2-1-HIGH-A: PASS]** `RemoveTagDiff::Removed(Tag)` payload 復元の検証 — `application.rs:31-46`

- `enum RemoveTagDiff { Unchanged, Removed(Tag) }` に payload 復活、`domain/workflows/remove-tag.md#steps` の `TagDiff = Unchanged | Removed(Tag)` と signature 整合
- `compute_diff` は `iter().find().cloned()` パターンで matched `Tag` を運搬。`Tag::clone` は O(1) 相当 (Arc 利用なら無コスト、`String` 1 個保持でも cheap)
- 呼出側 application.rs:77-80 で `let _matched = match ... { Removed(t) => t }` で受領し、aggregate `Note::remove_tag(&tag_name, now)` を呼ぶ
  - `_matched` を実際には使わず discard しているため「現時点で payload は使われていない」点は事実。ただし spec.md#impl-related-slices と workflow#steps の signature 一致は復元され、coherence の根本問題は解消
- spec.md#impl-related-slices もこの patch で `RemoveTagDiff = Unchanged | Removed` から `Unchanged | Removed(Tag)` に揃えるべきだが、impl 側 doc comment (application.rs:27 "Mirrors `TagDiff = Unchanged | Removed(Tag)` declared in `domain/workflows/remove-tag.md#steps`") で明示的に upstream 参照しているため、spec 側のラグは LOW 扱いに格下げ可能 (下記 P2-5-LOW-N 参照)

**[P2-1-MED-B: PASS]** spec.md `## Open Questions` セクション追加の検証 — `spec.md:196-222`

- 3 OQ が H3 + section id 付きで記録され、各 OQ に「理由 / 検討 / 現状」3 項目が揃っており propose/retain 判断材料として充分
  - `#oq-remove-tag-error-3variant`: 「BC 内 3 slice 共通 pattern として確立してから domain proposal」と保留理由が明示
  - `#oq-remove-tag-now-injection`: 「assign-tag と束ねた proposal を /ori-propose 経由で作る予定」と次アクション明示
  - `#oq-tag-name-type-strength`: 「cross-slice refactor として後送り」と scope 制約が明示
- 全 OQ が assign-tag slice の同型 OQ を cross-reference しており、Note Capture BC 全体の coherence 視点で扱う方針が読み取れる

**[P2-1-MED-D: PASS]** signature type-weakness の OQ 化検証 — `spec.md:216-222`

- `#oq-tag-name-type-strength` として記録、aggregate signature の `&str` は本 PR 維持・cross-slice refactor 後送りという判断を明示
- ただし aggregate `Note::remove_tag(self, tag_name: &str, now: Timestamp) -> Self` の signature は patch 後も変更なし (note.rs:84) で、OQ 記録のみ。これは Pass 1 推奨の「OQ 化して後送り」と合致

**[P2-1-MED-F: PASS]** `tp_nm2_empty_string_tag_name_yields_noop` 追加の検証 — `tests.rs:340-366`

- doc comment が「I-RT1 の境界ケース」「`Tag::name` は空文字を含み得ない (I-N6) ため必ず no-op」と理由まで明文化
- assertion は `result.is_none()` + `write_count == 0` + `event_count == 0` の 3 軸で I-RT2 を pin
- spec.md TP catalog には `tp-noop-missing TP-NM2` として位置づけ済 (test doc comment 内 `spec.md#tp-noop-missing TP-NM2`)。spec.md#tp-noop-missing 本文には TP-NM2 への explicit 言及が無いが、空文字 case の意味的所属は `#tp-noop-missing` (該当 tag が存在しないため) で正しい。これは LOW (P2-3-LOW-M 参照)

### 観点 2: regression 検出 {#pass2-findings-2-regression}

**[P2-2-PASS]** `RemoveTagDiff::Removed(Tag)` 変更による副作用 regression なし

- 17/17 slice tests GREEN を確認 (cargo test --lib note_capture::slices::remove_tag)
- `Tag` payload を application 層で discard (`let _matched`) しているため、aggregate call は patch 前と同じ `Note::remove_tag(&tag_name, now)` で behavior 同一
- `Tag::clone()` 呼び出しが no-op path 以外で 1 回追加されたが、Tag は軽量 value object で performance 影響なし
- `enum` variant に payload を持たせると Rust の memory layout が変わる可能性があるが、`RemoveTagDiff` は application.rs 内部の non-pub enum で外部 ABI 露出なし → 影響範囲は本ファイル内に閉じる
- clippy 警告は本 slice 内では発生していない (報告通り)。`let _matched` pattern は意図的な discard で `unused_variables` 警告も `_` prefix で回避済

### 観点 3: Open Questions セクション充足性 {#pass2-findings-3-oq-sufficiency}

**[P2-3-PASS]** 3 OQ は retain/propose 判断材料として充分

- 各 OQ に (理由・検討・現状) の triadic 記述があり、後続セッション (`/ori-propose` 起動時) で再判断するための context は揃っている
- assign-tag の同型 OQ への cross-reference が明示されており、cross-slice の束ね propose 戦略 (例: now-injection を 2 slice 同時に proposal) が読み取れる

**[P2-3-LOW-L]** OQ `oq-remove-tag-error-3variant` に「BC 内 3 slice 共通 pattern として確立してから」とあるが、現時点で既に auto-save-note / assign-tag / remove-tag の 3 slice が `LoadError` を持つ — `spec.md:200-206`

- 「establish 後に propose」の trigger 条件が既に満たされている可能性がある
- **推奨対応 (LOW)**: 次 `/ori-finalize remove-tag` 時に upstream proposal の起票を検討。LOW 扱いで本 PR は通過

**[P2-3-LOW-M]** spec.md#tp-noop-missing 本文に TP-NM2 (空文字 case) への explicit 言及が無い — `spec.md:106-108`

- test doc comment は `spec.md#tp-noop-missing TP-NM2` を参照しているが、spec.md#tp-noop-missing は `tag_name="python"` の case のみ記載。TP-NM2 (`tag_name=""`) が同 section にぶら下がる根拠を spec 本文で示すと test-spec トレーサビリティが完璧になる
- **推奨対応 (LOW)**: spec.md#tp-noop-missing の末尾に「TP-NM2: 空文字 `tag_name` も Tag::name 完全一致しないため同じ no-op 経路 (I-RT1 + I-N6 由来)」を 1 行追記。本 PR では LOW、次回派生時に同期される想定

### 観点 4: Pass 1 LOW 指摘の悪化チェック {#pass2-findings-4-low-regression-check}

**[P2-4-PASS]** Pass 1 LOW 指摘 (2-LOW-C / 4-LOW-E / 5-LOW-G / 5-LOW-H / 6-LOW-I / 7-LOW-J / 7-LOW-K) はいずれも患部変更なしで悪化なし

- 2-LOW-C: aggregates.md signature 不一致 → `oq-remove-tag-now-injection` で吸収済 (MED-B 解消の副産物)
- 6-LOW-I: tests.rs:571 のコメントは `// ===== TP-NU*: no event for unchanged =====` のまま (test prefix `tp_eu1_` との naming mismatch は未修正だが新規 regression ではない)。LOW のまま継続

### 観点 5: Pass 2 新規発見 {#pass2-findings-5-new}

**[P2-5-LOW-N]** spec.md#impl-related-slices の `RemoveTagDiff = Unchanged | Removed` 記述が patch 後の impl `Removed(Tag)` と未整合 — `spec.md:187`

- impl (application.rs:31-34) は patch で `Removed(Tag)` に変更されたが、spec.md#impl-related-slices 本文は `RemoveTagDiff = Unchanged | Removed` のまま
- HIGH-A patch では impl と upstream workflow を揃えたが、中間文書である本 spec の説明文の更新が漏れている
- impl の doc comment が upstream workflow を直接参照しているため、機能的 coherence は保たれているが、spec.md を単体で読んだ場合に「Pass 1 で指摘された矛盾が残っている」と誤読される
- **推奨対応 (LOW)**: spec.md:187 を `RemoveTagDiff = Unchanged | Removed(Tag)` に書き換える 1 字修正。本 PR では LOW 通過、次 `/ori-sync` または `/ori-derive` で吸収可能

**[P2-5-LOW-O]** `let _matched = match ... { Removed(t) => t, ... };` の `_matched` discard は payload 復元の意義を弱める — `application.rs:77-80`

- HIGH-A の修正動機 (a)(b)(c) のうち (b)「audit log で実際に消した Tag を識別」と (c)「log/event/UI 通知」の何れも現状未実装。payload は signature 整合性 (a) のみのために存在
- 現 impl は upstream contract を満たすが、将来の audit 拡張ポイントとして payload 利用例 (例: `tracing::debug!("removed tag: {}", _matched.name())`) があれば payload の有用性が示せる
- **推奨対応 (LOW)**: 必須ではない。次 slice (例: rename-tag) 着手時に audit log pattern を導入する際にまとめて検討

## Disposition Pass 2 {#disposition-pass-2}

**判定: PASS** (新規 HIGH/MED なし、LOW 3 件のみ)

### Pass 1 findings 解消状況 {#pass2-fix-summary}

| Finding | Severity | 解消方法 | 検証結果 |
|---|---|---|---|
| 1-HIGH-A | HIGH | impl を `Removed(Tag)` に変更 | PASS — workflow#steps と signature 一致、17/17 GREEN |
| 1-MED-B | MED | `## Open Questions` セクション + 3 OQ 追加 | PASS — propose/retain 判断材料 triadic 記述で充分 |
| 3-MED-D | MED | `oq-tag-name-type-strength` で OQ 化 | PASS — cross-slice refactor scope 制約明示 |
| 5-MED-F | MED | `tp_nm2_empty_string_tag_name_yields_noop` 追加 | PASS — assertion 3 軸で I-RT2 pin |

### Pass 2 新規 LOW {#pass2-new-low}

| ID | 概要 | 推奨対応 |
|---|---|---|
| P2-3-LOW-L | `oq-remove-tag-error-3variant` の "establish 後に propose" trigger 条件が既に充足 | `/ori-finalize` 時に upstream proposal 起票検討 |
| P2-3-LOW-M | spec.md#tp-noop-missing 本文に TP-NM2 (空文字) への explicit 言及なし | spec.md に 1 行追記 |
| P2-5-LOW-N | spec.md#impl-related-slices の `RemoveTagDiff` 記述が `Removed(Tag)` に未追従 | spec.md:187 を 1 字修正 (`Unchanged \| Removed(Tag)`) |
| P2-5-LOW-O | `let _matched` discard が payload 復元意義を弱める | 次 slice で audit pattern 導入時に再考 |

### Pass 2 ハイライト {#pass2-highlights}

- Pass 1 HIGH/MED 全 4 件が patch で適切に解消されている
- regression なし: 17/17 slice tests GREEN、payload 追加による副作用は本ファイル内に閉じる
- Open Questions の記述品質が高い (理由・検討・現状の triadic 構造 + cross-slice 参照) ため、後続 propose 判断が機械的に行える
- patch scope が最小限に絞られ、cross-slice 変更や over-engineering を避けた良い NEEDS_FIX → PASS の移行例
