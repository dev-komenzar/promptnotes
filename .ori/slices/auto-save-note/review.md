# Review: auto-save-note

## Pass 1

Reviewer: Claude Opus 4.7 (1M), capability=reasoning, fresh context
Scope: spec.md + manifest.yaml + 4 upstream domain docs + src under `apps/promptnotes/src-tauri/src/note_capture/`

### 7 観点

#### 1. spec ↔ impl 整合性

- pipeline (spec.md#impl-pipeline step 1..7) は `application.rs` がそのまま辿っており、step 4 が `BodyDiff::Unchanged → Ok(None)` の早期 return として表現されている。`BodyDiff` enum を `Unchanged | Changed(NoteBody)` の two-variant にして compile-time に「Unchanged 経路で write を呼べない」状態を作っているのは spec の意図に忠実。
- `Result<Option<Note>, AutoSaveError>` の戻り値型 (spec.md#io-output) は `application.rs:40` と `tests.rs:537-542` の signature pin で一致。
- ただし spec.md#io-errors が AutoSaveError variant を 2 件 (`NoteNotFound`, `PersistError`) と明記しているのに対し、impl は `InvalidBody` を追加しており**乖離**。後述 finding HIGH-1。
- spec.md#impl-tauri は「`Result<Option<Note>, AutoSaveError>`」を要求しているが、commands.rs では `AutoSaveOutcome` / `AutoSaveErrorDto` DTO に変換している。DTO 変換は frontend 側 serde 都合上妥当だが spec から逸脱しているので open question 化が望ましい (LOW-1)。

#### 2. derives_from 網羅

- manifest.yaml の 6 derives_from を確認：
  - `workflows/auto-save-note.md#auto-save-note` → pipeline 完全反映 ✅
  - `aggregates.md#note-aggregate` → `Note::edit_body` 経由で I-N1/I-N3/I-N4 を尊重 ✅
  - `bounded-contexts.md#note-capture` → Note Capture BC 配下に slice 配置、shared 拡張も BC 内で完結 ✅
  - `domain-events.md#note-body-edited` → `DomainEvent::NoteBodyEdited { note_id, updated_at }` payload 一致 ✅
  - `validation.md#s2-autosave-debounce` → tp_h1..h3 で Then 節を充足 ✅
  - `validation.md#s9-idempotent-autosave` → tp_i1..i3 で body 比較 → 早期 return を確認 ✅
- 網羅性は OK。

#### 3. DDD 規約遵守 / 層配置

- `domain.rs` は副作用ゼロ (Command / Error の型定義のみ)。
- `application.rs` が pipeline orchestration を担い、I/O は `repo.load_by_id` / `repo.write` / `bus.publish` / `clock.now` の port 越しのみ。pure code に I/O 混入なし ✅
- `commands.rs` のみが Tauri 依存 (`tauri::*`, `AppHandle`, `Runtime`)。Hex 境界は守られている ✅
- `infrastructure.rs` (create_note slice) に `load_by_id` 実装と `parse_note_md` を追加。これは create_note slice の責務拡張だが、`FsNoteRepository` は両 slice が共有する単一 impl なので合理的。ただし「create_note slice 配下に load 機能が常駐する」ことが将来読みづらい可能性 (LOW-2)。
- Result 型を throw で代用していないか → `expect("Tauri must resolve app_data_dir on supported platforms")` (commands.rs:72) と `expect("YYYYMMDDhhmmss formatting must not fail ...")` (timestamp.rs:32) は infallible 不変条件箇所のみ。`note_md_path` 等の正常系では `?` を貫いている ✅

#### 4. 副作用境界 / aggregate 不変条件

- C-AS5 (persist 失敗時 event 非発行) を application.rs 65-73 が線形に保証している ✅
- C-AS6 (event 1 回発行) は use case 内で 1 箇所のみ publish。テスト tp_h3, tp_pe3, tp_pe4 で確認 ✅
- I-N1 (id immutable) → `Note::edit_body` が `..self` で id を引き継ぐ + tp_inv1 で確認 ✅
- I-N3 (updated_at >= created_at) → `Clock` 契約として spec.md#invariants-note-aggregate で前提されており、impl 側で defensive 検査はしていない。これは spec 通り (Clock contract が壊れたら domain も壊れる) ✅
- I-N4 (秒精度更新) → `Timestamp::from_offset_datetime` が `replace_nanosecond(0)` で秒精度に強制。`SystemClock` / `FixedClock` 双方で保証されている ✅
- C-AS3 (body unchanged で early return) → `compare_body` が `&NoteBody == &NoteBody` を行い、Unchanged で `return Ok(None)`。tp_i2/tp_i3 が「write も publish も呼ばれない」を検証 ✅
- C-AS8 (tags 不変) → tp_h4 で `seed.tags()` と updated.tags() の等価を確認 ✅
- S9 idempotency: bus への発行は 1 度だけ。tests でも `write_count() == 0`, `event_count() == 0` を assert ✅

#### 5. テスト網羅 / 境界値

- TP-H1..H4, TP-I1..I3, TP-NF1..NF2, TP-PE1/PE3/PE4, TP-BC1..BC3, TP-INV1/INV2, TP-AS1 をカバー。
- **欠落**: TP-PE2 (`cause.kind() == ErrorKind::PermissionDenied`) が単独の `#[test]` で書かれていないが、tp_pe1 内で `assert_eq!(source.kind(), io::ErrorKind::PermissionDenied)` を行っているので実質充足 (MED-1: spec の TP-PE2 と test の対応コメントが不足、ただし機能的に OK)。
- **欠落**: TP-NF3 (`id` フィールドが入力の `note_id` をそのまま返す) は tp_nf1 内の `assert_eq!(id, missing)` で代行されている ✅
- **欠落**: TP-AS2 (tags 不変の type-level 確認) は実質 tp_h4 でカバー。spec.md#tp-api-shape の文言は「TP-H4 の延長で確認」を許容しているため OK ✅
- **TP-INV3** (秒精度連続編集) は spec 自身が「実際には TP-I1 ルートに行くため不要」と注釈しているので未実装 OK ✅
- **境界値テストの不足** (MED-2):
  - `parse_note_md` への異常入力 (key 重複 / 空 tags `[]` / body 末尾改行) の単体テストがない。`FsNoteRepository::load_by_id` の roundtrip テストもない。create_note slice 側で write の確認はあるが、read 経路は新規追加なのに直接テストされていない。Tauri 経由でしか経路を踏まず、回帰検知が弱い。
  - `parse_note_id` (commands.rs:107) の sentinel epoch 経路は明示テストなし。Tauri command を unit test するのは困難だが、`parse_note_id("garbage")` が `Timestamp::parse_yyyymmddhhmmss` の Err を踏むことの doc-test / unit test があれば S9 の noise を減らせる (MED-3)。

#### 6. テスト ↔ spec トレース

- 全 `#[test]` に `/// spec.md#tp-* TP-X` の doc-comment があり、トレース性は良好 ✅
- ただし tp_pe1 が TP-PE1 と TP-PE2 を兼ねていることへの言及がない (MED-1)。

#### 7. 冗長性 / premature abstraction

- `RcRepo` / `RcBus` の wrapper (tests.rs:111-129) は `Rc<FakeRepo>` に直接 NoteRepository を実装できないための必要悪。冗長ではない ✅
- `AutoSaveNoteUseCase` の 3 generic は overkill ではないか？ → `Clock` / `EventBus` の test double がそれぞれ必要なので generic は妥当 ✅
- `note_md_path` / `persist_error` の private helper は重複排除として妥当 ✅
- `NoOpBus` (commands.rs:27-32) は production で event を捨てる。spec.md#io-input は「`EventBus` — domain event の **同期** 発行」と書いており、production で publish が NoOp なのは spec 違反ではないが、本 slice の S2 検証の Then 節「event 発行」を満たす実装が存在しない。後続 slice (note_feed の subscribe) で繋ぐ予定らしいが、現状コードコメント以上の Tracking が不十分 (MED-4)。

### 既知 divergence への評価

1. **`AutoSaveError::InvalidBody` 追加** — `NoteBody::new` が `---` 行を reject する仕様 (note_body.rs:11-16) は事実なので、これを use case が握り潰さない選択は **正しい**。ただし対応は 3 つの選択肢があり、現状は最悪：
   - (a) spec を改訂 (`/ori-propose`) して `InvalidBody` を正規 variant に昇格
   - (b) Note Capture BC で「NoteBody は任意 UTF-8」を真にし `---` 制限を別 invariant に移す
   - (c) 現状維持 (impl コメントで spec との乖離を明示)
   - 現状の `domain.rs:11-32` のコメントは「spec.md の open question `oq-invalid-body-variant` 参照」と書いているが、**spec.md には `oq-invalid-body-variant` セクションが存在しない** (spec.md には `oq-invalid-note-id` と `oq-newline-normalization` の 2 つしかない)。コメントの参照先が壊れている → **MED-5 (NEEDS_FIX 推奨)**

2. **`parse_note_id` の sentinel epoch** — `NoteId::from_timestamp(UNIX_EPOCH)` を sentinel として `NoteNotFound` に降格する設計は spec.md#oq-invalid-note-id が「frontend は常に valid を送る前提なら集約」と明示しているので acceptable。ただし sentinel epoch が `19700101000000.md` という実在しうる ID と衝突する点は防衛的に弱い (LOW-3)。せめて `parse_note_id` が `Result<NoteId, AutoSaveError::NoteNotFound { id: NoteId(raw) }>` を返す方が clearer だが、`NoteId` に raw String constructor が無いため現状は妥協解として OK。

3. **改行コード正規化** — spec.md#oq-newline-normalization で open question 化済。impl はバイト等価 (spec 暫定方針通り)。問題なし。

4. **`FsNoteRepository::load_by_id` の自前 frontmatter parser** — `parse_note_md` (infrastructure.rs:93-154) のエッジケース：
   - **key 重複**: 最後の値が勝つ (Option を上書き)。明示的な reject ではないがクラッシュも起こさない。LOW
   - **空 tags `[]`**: `inner = ""`, `split(',').filter(|t| !t.is_empty())` で空 Vec → `TagSet::from_tags(vec![])` → empty TagSet。これは正しい ✅
   - **body 末尾改行**: `body_start` は閉じ `---\n` の直後の offset。write 側 (infrastructure.rs:48-49) は `---\n` + body をそのまま書く → body に末尾改行が入っていればそのまま preserve。これは roundtrip 性として正しい ✅
   - **`---` で始まらないファイル**: `MissingOpenDelimiter` で `InvalidData` → `PersistError` に化ける**バグ**：use case (application.rs:42-48) は `load_by_id` の Err を `persist_error` に変換しているため、**read 失敗が PersistError として表面化する**。これは spec.md#io-errors 違反。→ **HIGH-2 (NEEDS_FIX)**

### Findings

- **HIGH-1 (spec ↔ impl error variant)**: `AutoSaveError::InvalidBody` variant は spec.md#io-errors に存在しない。spec が「parse 失敗ケースがない」と断言しているが、共有 `NoteBody::new` が `---` 行を reject する以上嘘である。修正方針:
  - `/ori-propose` で spec を改訂し `InvalidBody { source: NoteBodyError }` を正規 variant に昇格、または
  - `aggregates.md#note-aggregate-elements` の「任意の UTF-8 文字列」記述を「frontmatter 区切り `---` を含まない UTF-8 文字列」に修正
  - どちらか上流側で正本化する必要あり。

- **HIGH-2 (read 失敗 → PersistError 誤分類)**: `application.rs:42-45` で `repo.load_by_id` の `Err` を `persist_error` で wrap している。spec.md#io-errors の `PersistError` 定義は「永続化失敗 (write 系)」を意図しており、`load_by_id` の I/O 失敗 (parse 失敗 / permission denied for read) を同じ variant に混ぜるのは意味論的に誤り。修正:
  - `load_by_id` の `Err` 経路用に別 variant (例: `LoadError`) を追加するか
  - `Ok(None)` のみを expected な「ノートがない」case として扱い、`Err` は panic で fail-fast (defensive 度合いに依存)
  - 最低でも spec.md に「load 失敗時の振る舞い」を明文化する。

- **MED-1 (TP-PE2 のテスト ID トレース不足)**: tp_pe1 内で `assert_eq!(source.kind(), io::ErrorKind::PermissionDenied)` は TP-PE2 相当だが、doc-comment が `TP-PE1` のみ。`TP-PE1 + TP-PE2` と並記が望ましい。

- **MED-2 (read path のテスト不足)**: `FsNoteRepository::load_by_id` + `parse_note_md` の roundtrip / 異常入力テストが存在しない。次 slice (`flush-note` 等) で踏むまで regression を検知できない。最低限「write → load → 比較」の roundtrip test を 1 本追加すべき。

- **MED-3 (`parse_note_id` の sentinel テスト不足)**: invalid raw → sentinel epoch NoteId → load miss → NoteNotFound の経路に明示テストなし。

- **MED-4 (`NoOpBus` の存在感の薄さ)**: production で event を捨てている事実が `commands.rs:27-32` のコメント 1 行のみで documented。beads issue / spec.md#impl-notes に「note_feed slice が landing するまで NoOp」を明記し、後で grep できる TODO marker を残すべき。

- **MED-5 (spec ref が壊れている)**: `domain.rs:18-20` のコメント「See open question `oq-invalid-body-variant` in spec.md」だが spec.md にそのアンカーは存在しない。spec.md#open-questions に `oq-invalid-body-variant` を追加するか、コメントを既存 ID に修正する必要がある。

- **LOW-1 (spec.md#impl-tauri の戻り型と impl DTO の乖離)**: spec は `Result<Option<Note>, AutoSaveError>` を要求しているが impl は `AutoSaveOutcome`/`AutoSaveErrorDto`。frontend serde 都合と spec の整合性を spec 側で update すべき (Note 型の serde 露出を避ける意図なら明示)。

- **LOW-2 (`parse_note_md` の slice 配置)**: 将来 `flush-note` slice も `load_by_id` を踏むため、`create_note` slice 配下に read parser が住んでいるのは違和感がある。`note_capture/shared/` 配下に `fs_note_repository.rs` として昇格する余地あり (refactor phase 候補)。

- **LOW-3 (sentinel epoch の衝突)**: `19700101000000.md` という極端 ID が実在する場合、`parse_note_id("garbage")` の sentinel と衝突する。実害は限定的だが、`NoteId` に validating constructor を入れて `Result` 化するのが正道。

### 総合判定

**NEEDS_FIX**

理由:
1. **HIGH-1**: AutoSaveError variant が spec と乖離。spec か aggregate 定義の片方を改訂し、`InvalidBody` の正本位置を確定する必要がある (1 pass で済む修正)。
2. **HIGH-2**: `load_by_id` の I/O 失敗が `PersistError` に化ける。意味論違反 + spec 違反。`load_by_id` の Err を別 variant に分けるか、spec.md#io-errors を「load/persist 両方を含む」と再定義する。
3. **MED-5**: `domain.rs` の spec 参照 (`oq-invalid-body-variant`) が壊れリンク。HIGH-1 の対応の一部として spec.md#open-questions に追加すれば併せて解決可能。
4. それ以外 (MED-2..4, LOW-1..3) は次 slice (`flush-note`) で踏む時に併せて手当て可能だが、HIGH-1/HIGH-2 は本 slice 内で閉じるべき構造的 issue。

S9 idempotency / I-N4 / 副作用境界 (C-AS5, C-AS6) の中核は正しく実装されており、テストも適切に観点を踏んでいる。総じて骨格は健全だが、error 表面の type 系統に spec 違反が 2 点ある状態。

---

## Pass 2

Reviewer: Claude Opus 4.7 (1M), capability=reasoning, fresh context
Scope: Pass 1 で挙げた HIGH-1 / HIGH-2 / MED-1 / MED-2 の修正後 diff (`git diff HEAD`) と spec.md / impl 一式 + 既知の cargo test (74/74) と clippy clean を確認。

### Pass 1 指摘の解消状況

#### HIGH-1 (`InvalidBody` variant の spec ↔ impl 乖離)

- 対応: spec.md#io-errors に `InvalidBody` / `LoadError` / `PersistError` の variant 一覧を直接書き換え、spec.md#open-questions に `oq-invalid-body-variant` を新設、domain.rs のコメント参照リンクは valid に。
- 評価: **部分的に解消されているが、CoDD 流儀との衝突あり (Pass 2 新規 HIGH-3、後述)**。

#### HIGH-2 (read I/O 失敗が `PersistError` に化ける)

- 対応: `AutoSaveError::LoadError { path, source }` variant 追加 (domain.rs:29-34)、`application.rs::load_error` helper (86-91) で wrap、`commands.rs` の `AutoSaveErrorDto::LoadError` (49) 追加、spec.md C-AS1 (slice spec の 123 行目) で `Ok(None)` / `Err(io)` を分岐、テスト `tp_le1_load_failure_surfaces_as_load_error_not_persist_error` (tests.rs:453-483) で regression を網羅。
- 評価: **十分。** semantic はクリアで、test も「LoadError 経路」「PersistError ではない」「write/publish 非発火」の 3 点を 1 件で押さえている。書込側 (`PersistError`) と意味的に分離する設計判断は health。

#### MED-1 (TP-PE2 のトレース)

- 対応: 関数名 `tp_pe1_pe2_write_failure_surfaces_as_persist_error_with_kind` + doc コメント `/// TP-PE1 + TP-PE2 (path + cause.kind() の両方を確認)`。
- 評価: **十分。**

#### MED-2 (load_by_id の roundtrip テスト欠落)

- 対応: `create_note/tests.rs` の末尾セクション (646-724) に 4 件追加:
  - `fs_note_repo_load_by_id_roundtrips_a_freshly_written_note` — write → load 完全 roundtrip (body / tags / created_at / updated_at)
  - `..._returns_none_on_missing_file` — `Ok(None)` 経路
  - `..._yields_invalid_data_on_malformed_frontmatter` — `---` 無しファイルで `InvalidData`
  - `..._handles_empty_tags_inline` — 空 tags の round-trip
- 評価: **十分。** spec.md#oq-* を踏まないエッジ (key 重複) は未カバーだが、これは MED 残として deferred 妥当。

#### 附帯発見 (Timestamp parse バグ修正)

- 対応: `OffsetDateTime::parse` (format に offset 無し、failure-prone) → `PrimitiveDateTime::parse + assume_offset(UtcOffset::UTC)` (timestamp.rs:39-41)。
- 副作用範囲: `parse_yyyymmddhhmmss` の呼び出しは `auto_save_note/commands.rs:113` (Tauri parse) と `create_note/infrastructure.rs:120,128` (frontmatter parse) のみ。`user_preferences/` には呼び出しなし (grep で確認)。
- 評価: **load-settings 含めて他 BC への regression なし**。74/74 GREEN + clippy clean を再現確認済み。

### Pass 2 新規 Findings

#### 1. spec ↔ impl 整合性 / CoDD 規約遵守

- **HIGH-3 (spec.md を upstream 整合性 review なしで impl 寄りに書き換えた)** `.ori/slices/auto-save-note/spec.md`:88-104

  Pass 1 の HIGH-1 (spec が impl と乖離) に対し、本 round-trip で取られたのは:
  - spec.md#io-errors の `enum AutoSaveError { ... }` ブロックを **「impl が実際に返す 4 variant」に直接書き換え** (`InvalidBody` / `LoadError` 追加、`PersistError` の `cause` フィールド名を `source` に変更)
  - 各 variant に bullet 解説を追記し、spec.md#oq-invalid-body-variant を新設
  - **`coherence.source: derived`** と先頭 banner (`> This file is a derived document. ... Use /ori-sync --force if you need to edit here directly; ori will create a proposal for the upstream review.`) は残置
  - `last_derived: 2026-06-25` と `hash:` の値も不変 (git diff で frontmatter 変更ナシ)

  問題:
  - **upstream `.ori/domain/workflows/auto-save-note.md#errors` は `{ NoteNotFound, PersistError }` のまま** (`grep` で確認)。aggregate `aggregates.md#note-aggregate-elements` の「任意の UTF-8」も未変更。
  - つまり「spec は SSoT に対し dirty な状態のまま、impl に合わせて手動上書きされた」状態。`/ori-propose` で `domain/workflows/auto-save-note.md#errors` への upstream 修正提案を作るのが CoDD 流儀のはず。
  - 自身が新設した `oq-invalid-body-variant` 内で「本来は `/ori-propose` で aggregates.md / workflow.md に upstream 修正を提案する」と認めているにもかかわらず、その手続きを踏まずに先に spec を impl に寄せている → 矛盾。
  - banner には「Use `/ori-sync --force` if you need to edit here directly; ori will create a proposal for the upstream review」と明記。spec.md の直編集経路は `/ori-sync --force` 通過必須だが、その痕跡 (proposal entry) が `.ori/` ツリーに見当たらない (`ls .ori/slices/auto-save-note/` は manifest / spec / status / notes / review のみ)。

  推奨:
  - `coherence.source` と banner を残したままなら、spec.md の `io-errors` セクション本文を **upstream の 2-variant に戻し**、`InvalidBody` / `LoadError` は `oq-*` セクション側のみで「open question として表面化」として記述する。本文 enum 定義の改変は upstream 改訂後にのみ行う。
  - もしくは `/ori-propose` を回して `.ori/proposals/` (or 該当 mechanism) にエントリを作り、`coherence.source` を一時的に `dirty-edited` 等に降格して人間 review 待ち状態に持ち込む。
  - 最低でも `oq-invalid-body-variant` の冒頭で「**本文 #io-errors は本 open question の結論を impl に寄せて先行記述しているが、upstream は未改訂**」と免責を明示する。現状の oq セクションは「impl は防衛で variant 追加済」と書くが、spec 本文も書き換え済みのことには触れない。

- **HIGH-4 (spec.md の internal inconsistency)** `.ori/slices/auto-save-note/spec.md`:124 vs 95-102

  C-AS2 が `NoteBody::from(String) で構築する（infallible、空文字も valid）` のまま残っている。一方で同じ spec の #io-errors は `InvalidBody { source: NoteBodyError }` を追加し、application.rs:52-53 は `NoteBody::new` (fallible) を使う。**C-AS2 自体が嘘になっている**。
  - HIGH-3 の影響波及: 上流改訂を踏まず impl 寄りに書き換えると、副次的に C-AS* の不変条件群も整合性を取り直す必要がある。今回は io-errors と oq だけ追記したため、invariants セクションが矛盾を保持。
  - 推奨: C-AS2 を「`NoteBody::new` (fallible) で構築する。`---` 行を含む場合は `AutoSaveError::InvalidBody` (#io-errors) を返す」に改訂、もしくは HIGH-3 を「本文は upstream に揃える」方向で解決すれば C-AS2 は inflectious のままで OK。

- **MED-6 (Pipeline 表記の更新漏れ)** `.ori/slices/auto-save-note/spec.md`:226

  spec.md#impl-pipeline step 2 は `parse_body: String → NoteBody — NoteBody::from(String) (infallible)` のまま。impl は `NoteBody::new` (fallible) に変わっており、ここも HIGH-4 と同根の漏れ。本文 #io-errors を直書きしたなら、impl-pipeline の type 表記も `String → Result<NoteBody, NoteBodyError>` に揃えないと spec 内整合が崩れる。

#### 2. derives_from 網羅

- manifest.yaml の derives_from には変更なし。Pass 2 で追加された `LoadError` variant の論理的根拠は `domain/workflows/auto-save-note.md#errors` ではなく **review HIGH-2 自身**であり、upstream には記述が無い。HIGH-3 の解決経路の一部として upstream に反映するか、もしくは `manifest.yaml` の `derives_from` から外して `review-derived: true` 等 marker を付ける検討が必要。今回は HIGH-3 で包括的に扱う。

#### 3. DDD 規約 / 副作用境界

- application.rs の構造 (load → parse → compare → branch → edit → persist → emit) は Pass 1 同様 health。`load_error` / `persist_error` private helper の重複排除も妥当。
- domain.rs の `AutoSaveError::LoadError` のドキュメンテーション (26-28) は明確で、PersistError との semantic 差を明示している。
- commands.rs の DTO に `LoadError` を漏らさず追加している (49)。
- 副作用混入 / Result の throw 代用 / I/O 層越境はいずれも未検出。Pass 1 評価を維持。

#### 4. テスト ↔ spec トレース

- 新規 `tp_le1` の doc-comment は `/// spec.md#io-errors LoadError: load_by_id が Err を返した場合`。spec.md には `tp-load-err` のような独立 section 番号がない (test-perspectives セクションには LE 系を追加していない)。`#io-errors` 直接参照は一応 valid だが、test-perspectives セクションに `TP-LE1` を切るのが網羅性として望ましい (LOW-4)。

#### 5. 冗長性 / refactor 候補

- `RcRepo` に `fail_next_load` proxy (tests.rs:121-123) が追加。これは tp_le1 が `RcRepo(repo.clone()).fail_next_load(...)` の経路で fake を制御するため必要。LOW: 既存 `RcRepo` impl が拡張されたのは妥当だが、test の中で `RcRepo(repo.clone())` を新規に作ってから `fail_next_load` を呼ぶ書き味は `repo.fail_next_load(io::ErrorKind::PermissionDenied)` だけで済む (`FakeRepo::fail_next_load` を Rc<FakeRepo> 越しに呼べる) ため、proxy method は冗長。実害はないので LOW のまま。

#### 6. 新規 regression / 副作用

- `Timestamp::parse_yyyymmddhhmmss` 修正の副作用: 入力フォーマットに timezone 列が無い前提を **明示的に** `assume_offset(UtcOffset::UTC)` で UTC に pin。`format_yyyymmddhhmmss` も timezone 列を持たないため、`format → parse → format` の roundtrip 性が保たれる。
- 影響先 (`auto_save_note::commands::parse_note_id` と `parse_note_md::createdAt/updatedAt`) いずれも UTC 前提で生成された文字列を読み戻すので、semantics は一貫している。
- load-settings (`user_preferences`) は `Timestamp` を import していない (grep で確認)。**他 slice への regression 無し**。

### Findings 一覧 (Pass 2)

- **HIGH-3**: spec.md (`coherence.source: derived` 残置) を `/ori-propose` 無しで impl 寄りに直編集 → CoDD SSoT 違反。spec.md banner 自身が「proposal for upstream review」を要求しており、その動線をスキップしている。
- **HIGH-4**: spec.md 内 C-AS2 と #io-errors / impl の semantic 不整合。NoteBody が fallible になったのに C-AS2 は infallible と書いたまま。
- **MED-6**: impl-pipeline step 2 の type 表記が C-AS2 と同根の更新漏れ。
- **LOW-4**: test-perspectives セクションに `TP-LE*` (LoadError) を追加し、`tp_le1` の doc-comment を `TP-LE1` 参照に揃えるのが網羅性として綺麗。

(Pass 1 で挙げた MED-3 / MED-4 / LOW-1..3 は本 round-trip では deferred 妥当として扱う、新規 finding ではない。)

### 総合判定

**NEEDS_FIX**

理由:
1. **HIGH-3 が新規 HIGH** — Pass 1 の HIGH-1 を解消する過程で「spec を impl に合わせて直編集する」という CoDD 流儀違反の手段が選ばれた。spec.md は今もなお `coherence.source: derived` を宣言しており、banner も「直編集は `/ori-sync --force` で proposal を作れ」と明示している。proposal entry の生成痕跡が `.ori/` に見当たらない以上、SSoT (upstream domain doc) を素通りした sneak change が spec.md 本文に入っている状態は finalize 前に解消すべき。
2. **HIGH-4 が新規 HIGH** — HIGH-3 の副作用として spec 内 invariant (C-AS2) と io-errors / impl が矛盾。slice 内で一読すると不変条件と error 表面の整合性が破綻する。
3. Pass 2 ルール (新規 HIGH 検出時は human flag を立ててループ停止) を発動する状況。`/ori-propose` 動線に乗せるか、もしくは spec.md 本文を upstream に戻して oq-* セクション側のみで divergence を documentation する方針を **人間が選択** する必要あり。

なお Pass 1 の HIGH-2 / MED-1 / MED-2 は適切に解消されており、impl + test の品質は finalize レディな水準。問題は spec / domain doc 系の coherence ハンドリングのみ。

> **Human flag**: HIGH-3 は impl の正しさではなく **CoDD process gate の越境** に関する判断。ori workflow の意図 (`/ori-propose` を介した upstream-first な改訂) を採るか、当面の expedience として spec を更新したまま finalize に進めるかは、design ownership ある人間の判断事項。reviewer agent としては「現状の spec.md 直編集 + `coherence.source: derived` 残置」の組み合わせは矛盾を抱えていると指摘するに留める。

---

## Pass 3

Reviewer: Claude Opus 4.7 (1M), capability=reasoning, fresh context
Scope: Pass 2 で挙げた HIGH-3 / HIGH-4 を解消するために取られた upstream-first パス
（proposal 2 件 accept + `/ori-derive auto-save-note` で spec.md 再生成 + impl 微修正 +
test TP-IB1 / TP-IB2 追加）の最終状態を fresh context で再評価。

材料:
- `.ori/proposals/accepted/2026-06-25-auto-save-note-{workflows,aggregates}-*.md`
- 改訂後 `.ori/domain/workflows/auto-save-note.md` / `.ori/domain/aggregates.md#note-aggregate`
- 再生成後 `.ori/slices/auto-save-note/spec.md` (frontmatter hash + 本文)
- impl: `auto_save_note/{domain,application,tests}.rs` + `shared/types/{note,note_body}.rs`
- create-note slice 側の roundtrip テスト群
- cargo test --lib: **76 passed; 0 failed** (auto-save 含む全 BC、Pass 2 期 74 + TP-IB1/IB2 で +2)

### Pass 2 指摘の解消状況

#### HIGH-3 (spec 直編集 / CoDD SSoT 違反)

- 対応経路:
  1. **Proposal 1** (`workflows-auto-save-note-errors`) を accept → `domain/workflows/auto-save-note.md#errors`
     を **2 variant → 4 variant** に正本改訂、`#notes` に NoteBody / load/persist 分離の補足を追記
  2. **Proposal 2** (`aggregates-note-aggregate`) を accept → `domain/aggregates.md#note-aggregate-elements`
     の `NoteBody` 定義に `NoteBody::new` smart constructor 規約を追記、`#note-aggregate-invariants`
     に **I-N8** を新設、`#note-aggregate-commands` に `Note::from_persisted` を追加
  3. `/ori-derive auto-save-note` で spec.md を再生成。`coherence.last_derived: 2026-06-25` /
     `hash.domain/workflows/auto-save-note.md#.*: 642c5094fd1a` /
     `hash.domain/aggregates.md#.*: 9f9048f5816b` に更新（Pass 2 期: `1a3b1789524f` /
     `94b27e21aade` から進行）
  4. spec.md banner (`This file is a derived document...`) は維持

- 評価: **十分に解消**。
  - upstream → derived の単方向 flow が確立した。spec.md は domain doc から hash-tracked に derive される
    純粋な派生物に戻り、`coherence.source: derived` 宣言と本文が再度一致した
  - `.ori/proposals/accepted/` 配下に accept 痕跡 (frontmatter に `accepted_at: 2026-06-25`,
    `accepted_by: human (takuya.kometan@gmail.com)`, `applied_to: ...` を明記) が残っており、
    decision audit log として完備
  - Pass 2 が指摘した「proposal entry の痕跡が `.ori/` ツリーに見当たらない」は完全に否定された
    (`accepted/` に 2 件、frontmatter の `target` / `applied_to` が一致)

#### HIGH-4 (spec.md 内 C-AS2 と #io-errors / impl の不整合)

- 対応箇所: spec.md L127 (C-AS2) と L237 (impl-pipeline step 2) が両方とも **fallible smart constructor**
  記述に統一された:
  - C-AS2: 「`new_body` 文字列を `NoteBody::new(String) -> Result<NoteBody, NoteBodyError>` で構築する
    （aggregate I-N8 由来の **fallible smart constructor**）。失敗時は `AutoSaveError::InvalidBody { source }`
    で表面化」
  - impl-pipeline step 2: 「`parse_body: String → Result<NoteBody, AutoSaveError>` —
    `NoteBody::new(String)` (**aggregate I-N8 由来の fallible smart constructor**)、失敗時は `InvalidBody`」
  - #io-errors の 4 variant 定義 (L92-98)、impl `application.rs:52-53`、test `tp_ib1` の三者が
    NoteBody fallibility に揃っている

- 評価: **十分に解消**。3 箇所すべてが「fallible」「InvalidBody で surface」で揃い、spec を 1 read して
  矛盾を引き起こす経路はなくなった。

### Pass 3 新規 Findings (なし) と附帯確認

#### 1. spec ↔ impl 整合性

- spec.md#io-errors の 4 variant ↔ `domain.rs:11-37` ↔ workflow upstream の 4 variant が完全一致
  (`NoteNotFound` / `InvalidBody` / `LoadError` / `PersistError`)。フィールド名も統一 (`source`)
- pipeline step 1..7 が `application.rs:40-77` と 1:1 対応。step 4 の `BodyDiff::Unchanged` 早期 return
  も spec の通り
- `Note::from_persisted` は `infrastructure.rs:153` から呼ばれており、aggregates.md の Commands 規約
  「永続化済 Note の再構築（NoteRepository::load_by_id 経由のみ）」と一致 ✅
- `NoteBody::new` の I-N8 enforce (`note_body.rs:11-16`) が aggregates.md#note-aggregate-elements
  の `NoteBody::new(raw: String) -> Result<NoteBody, NoteBodyError>` 規約と一致 ✅

#### 2. derives_from 網羅 (再評価)

- manifest.yaml の 6 derives_from エントリと spec.md 本文の `>` 引用が正確に対応している:
  - `workflows/auto-save-note.md#auto-save-note` → spec.md L31, L46, L91, L106
  - `aggregates.md#note-aggregate` → spec.md L33 (subdomain-type 引用に bounded-contexts.md 経由),
    L115-120 (I-N1/I-N3/I-N4/I-N8 invariant), L127 (NoteBody smart constructor)
  - `bounded-contexts.md#note-capture` → spec.md L33
  - `domain-events.md#note-body-edited` → spec.md L77-84
  - `validation.md#s2-autosave-debounce` → tests tp_h1..h3 で Then 節充足
  - `validation.md#s9-idempotent-autosave` → spec.md L37 + tests tp_i1..i3
- 網羅性 OK。新たに `derives_from` の補足が必要なほどの上流 (review-derived) 要素もない
  (Pass 2 で指摘した `LoadError` の根拠が review HIGH-2 → workflow upstream に正本化済み)

#### 3. Aggregate doc 変更の波及確認 (Proposal 1 周り)

- aggregates.md#note-aggregate-invariants:
  - 既存 I-N1..I-N7 (Tag 系 I-N5/I-N6 含む) と新規 I-N8 がコンフリクトなし
  - I-N8 は **body 構築規約**で、Tag 不変条件 (I-N5/I-N6) は **tags 構築規約**。同居しても干渉なし
- aggregates.md#note-aggregate-commands:
  - `Note::create` (既存) + `Note::from_persisted` (新規) は **identity 起源の差**を明示:
    - `create`: `id = now.format(YYYYMMDDhhmmss)`, `createdAt = updatedAt = now`
    - `from_persisted`: `id = NoteId::from_timestamp(created_at)` で I-N2 を construction-time に保証
  - `Note::from_persisted` の引数順 (body, tags, created_at, updated_at) と impl `note.rs:31-44`
    のシグネチャが完全一致
- aggregates.md の `## Open Questions` (L277-283) は触られておらず、aggregate level の未解決事項に
  噪 (noise) 持ち込んでいない

#### 4. Workflow doc 変更の波及確認 (Proposal 2 周り)

- workflows/auto-save-note.md#errors の 4 variant が `domain.rs::AutoSaveError` と完全一致
- #notes に補足 4 件:
  1. 「同一秒内の連続編集は `updated_at` が変わらない」(既存)
  2. 「`DebounceTimer` の cancel は flush-note workflow の責務」(既存)
  3. 「`NoteBody` 不変条件 (I-N8) は aggregate 由来。AutoSave は新規 body を受け取り構築するため、
     aggregate と同じ smart constructor を通る」(新規、Proposal 2 反映)
  4. 「read 失敗 (`LoadError`) と write 失敗 (`PersistError`) は意味的に異なる経路として variant 分離」
     (新規、Proposal 2 反映)
  5. 「同じ error 分類は `flush-note` workflow にも同形で適用すべき」(新規 followup TODO)
- spec.md L106 に「read 失敗（`LoadError`）と write 失敗（`PersistError`）は意味的に異なる経路として」が
  workflow#notes から正確に引用されている ✅

#### 5. テスト網羅 / 新規 test 追加 (TP-IB1 / TP-IB2)

- spec.md#tp-invalid-body (L174-175) に **TP-IB1 / TP-IB2** が新設され、test 側 `tp_ib1` (L489-511) /
  `tp_ib2` (L515-530) が 1:1 対応。doc-comment も `/// spec.md#tp-invalid-body TP-IB1` で正確
- TP-IB1 は `before\n---\nafter` を入力に `InvalidBody { source: ContainsFrontmatterDelimiter }` を assert。
  NoteBodyError variant マッチも含む厳格な test
- TP-IB2 は `"---"` 単体入力で `write_count == 0 && event_count == 0` を assert (C-AS5 と同形の副作用ガード)
- Pass 2 で指摘した **LOW-4** (`tp_le1` の doc-comment が `TP-LE1` を参照していない) について確認:
  spec.md には `tp-load-err` セクション (L167-170) として TP-LE1/LE2/LE3 が存在するが、test 側
  `tp_le1` の doc-comment は `/// spec.md#io-errors LoadError: load_by_id が Err を返した場合` のまま。
  これは LOW のままで finalize 後の improvement 候補 (LOW-5 — 後述)
- 76/76 GREEN: auto-save-note slice の test 群 (16 件) + create-note slice の test 群 (33 件、
  fs_note_repo_load_by_id roundtrip 4 件含む) + user_preferences (27 件) で **regression なし**

#### 6. DDD 規約 / 副作用境界 (Pass 1/2 評価維持)

- `application.rs` の load → parse → compare → branch → persist → emit が pure orchestration、I/O は
  port 越しのみ
- `domain.rs` の `AutoSaveError::InvalidBody` doc-comment (L15-16) が「**defensive variant**」表記から
  「`NoteBody::new(new_body)` failed (aggregates.md#note-aggregate-invariants I-N8: body must not
  contain a frontmatter delimiter line)」に修正済 — divergence 表記が消え、aggregate doc 由来の
  正本 invariant として記述される (Pass 2 のレビュー流れと一致)
- Result 型を `expect` で代用していないか確認: `tests.rs:163` / `commands.rs:72` (Pass 1 で infallible
  正当化済), `timestamp.rs` (Pass 1 評価維持)。新規追加なし

#### 7. flush-note workflow への波及 TODO

- `domain/workflows/auto-save-note.md#notes` に「同じ error 分類は `flush-note` workflow にも同形で
  適用すべき (accept 時の確認事項として記録、`flush-note` 派生時に再確認)」と明記
- proposal 2 `frontmatter.followup` にも「flush-note workflow にも同形を将来適用 (本 proposal で
  同時改訂はしない)」と記載
- 現状 `domain/workflows/flush-note.md#errors` は旧 2 variant (`NoteNotFound` / `PersistError`) のまま
- 評価: **acceptable**。flush-note は別 slice であり、本 slice の責務外。auto-save-note 内の `out-of-scope`
  にも明記されている (spec.md L275)。flush-note slice が起こる時に同形提案を再評価する設計で十分
- ただし flush-note 派生時に必ず想起させる仕組みとして、auto-save-note workflow doc + proposal followup
  の 2 箇所に痕跡が残っているので、忘却リスクは低い

#### 8. 既存 slice / 既存 BC への regression

- create-note slice: `NoteBody::new` の fallibility は本 slice 以前から導入されており、create-note slice
  の I-N6 (now: aggregate doc から見ると I-N8) 由来 test も `NoteBody::new(...).unwrap()` で対応
- load-settings slice: `user_preferences` BC は `NoteBody` / `Note` を import しておらず、Proposal 1/2
  の波及範囲外
- cargo test 76/76 GREEN がこれを実証

#### 9. open question (`oq-invalid-note-id` / `oq-newline-normalization`) の扱い

- 両方とも spec.md#open-questions に `status: open` で残置 (L286-303)
- `oq-invalid-note-id`: NoteId smart constructor が未整備で sentinel epoch 経由 `NoteNotFound` 降格を
  現 impl が選択。spec.md は「frontend は常に valid を送る前提」と明記し、問題顕在化後に proposal を
  作る運用方針
- `oq-newline-normalization`: バイト等価で実装、改行正規化は未着手。spec.md は「問題顕在化後に
  domain 側 (S9 補足) に提案」と運用方針を明記
- いずれも HIGH を生むレベルの設計欠陥ではなく、open question として spec で documentation されており、
  finalize の阻害要因ではない。Pass 1 で **LOW-3** (`19700101000000.md` との衝突) として既出だが、
  本 slice 内で閉じる必然性なし

### Findings 一覧 (Pass 3)

- **(なし)** Pass 2 で挙げた HIGH-3 / HIGH-4 はいずれも **upstream-first パス** (proposal accept +
  re-derive) によって構造的に解消された。新規 HIGH も検出されず
- **(LOW-5、新規)**: spec.md#test-perspectives の `tp-load-err` (TP-LE1/LE2/LE3) が定義されているのに
  対し、test 側 `tp_le1` の doc-comment は `/// spec.md#io-errors LoadError: ...` のまま。
  `TP-LE1` 参照に揃えるとトレース性が改善 (Pass 2 LOW-4 の再掲、finalize 後 refactor 候補)
- **(LOW-6、新規)**: Pass 1 で挙げた **LOW-2** (`parse_note_md` が create_note slice 配下にある点)
  は本 Pass で改善されていない。本 slice の責務外なので OK だが、flush-note slice が landing する
  時の refactor phase 候補として残す

### 総合判定

**PASS**

理由:

1. **HIGH-3 が構造的に解消** — Pass 2 の指摘 (spec 直編集 + `coherence.source: derived` 残置の矛盾)
   は upstream-first パスで完全に解決された。proposal 2 件 accept → domain doc 改訂 → `/ori-derive`
   で spec 再生成 → frontmatter hash 更新、という ori workflow の正規動線を踏破した痕跡が
   `.ori/proposals/accepted/` と spec.md frontmatter の両方に残っている。CoDD の SSoT 構造が回復した
2. **HIGH-4 が構造的に解消** — spec.md 内部の C-AS2 / impl-pipeline / #io-errors / impl / test の
   **5 経路すべて**が「`NoteBody::new` (fallible) → `InvalidBody` で surface」で一貫している。
   1 read で矛盾を引き起こす経路はゼロ
3. **新規 HIGH なし** — Pass 3 で fresh context で見直しても、HIGH レベルの構造問題は検出されない。
   `Note::from_persisted` の追加、I-N8 invariant、4 variant errors、TP-IB1/2 test がすべて
   coherent な 1 つの design 改訂として帰着している
4. **regression なし** — cargo test 76/76 GREEN。create-note / load-settings 両 BC への波及なし
5. **flush-note への TODO は acceptable** — proposal followup と workflow#notes の 2 箇所に痕跡が
   あり、flush-note slice 派生時に同形 (4 variant errors) を再評価する運用が確立。本 slice 内で
   閉じる必要なし
6. **open question 2 件 (`oq-invalid-note-id` / `oq-newline-normalization`) は documented**
   される open question として finalize 後も継承可能

`/ori-finalize auto-save-note` への移行 OK。LOW-5 / LOW-6 は finalize 後の refactor タスク
(beads issue 起票) として残すのが妥当。

> **Note (Pass 3 reviewer 所感)**: Pass 2 が「HIGH-3 を解消するのに 2 つの選択肢 (a) `/ori-propose`
> パス (b) spec を upstream に戻して oq 側に逃がす を人間判断」と問いを残した状況で、
> **(a) upstream-first パス** を選択して domain doc 側を改訂したのは ori workflow の意図と完全に
> 一致する正解。CoDD memory の「[Spec is the source of truth](feedback_spec_is_source_of_truth.md)」
> も派生方向のみならず「乖離発見時はどちらを直すかをユーザーに確認する」原則を要求しており、
> 本ケースはその原則に従って upstream を正本化した形 — design ownership の判断として健全。
