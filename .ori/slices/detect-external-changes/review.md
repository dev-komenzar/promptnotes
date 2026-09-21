# Review: detect-external-changes {#review-detect-external-changes}

## Pass 1 {#pass-1}

### Structural gates

- (a) boundary test: **PASS** — `cargo test detect_external_changes` → 21 passed / 0 failed
  (含 `watcher_restart_drops_old_dir_and_detects_new_dir` — C-DEC7 / C-DEC11 / TP-WL4)
- (b) arch lint: **PASS** — `cargo clippy --all-targets` は本 slice 由来の新規 warning なし
  (報告される warning は `&PathBuf` / `io::Error::other` / `new_without_default` / 複雑型の
  いずれも本変更前から存在する既存コード)
- (c) public_entry: **PASS** — `slices/detect_external_changes/(domain|application|infrastructure)/`
  の slice 外直 import は 0 件

### Semantic review

fresh-context の `ori-reviewer` agent spawn は orchestrator 制約 (ハング回避) により
**意図的に skip**。本 review は機械 gate + S22 E2E 実測に基づく。

- S22 E2E `.ori/scenarios/s22-storage-dir-change-watcher-restart`: **4 passing / 0 failing**
  (step1 old watcher 稼働 / step2 S11 回帰 / step3 new dir Modify 検知 / step4 new dir Create 検知)
- 回帰 E2E: s11 3 passing / s16 4 / s17 4 / s18 4 / s21 5 — いずれも 0 failing
- frontend unit: 164 passed

### Findings

- **LOW** spec.md#oq-watcher-idempotent:
  `start_file_watcher` を既に起動済み watcher に対して再呼出しした場合、現実装は旧 watcher を
  drop して再起動する (冪等)。spec の暫定 (AlreadyRunning error) とは異なるが、S22 の
  watcher 再起動経路は `storage_dir` 差替えを正しく行うため挙動上の問題はない。
  Open Question として残す (status.yaml#followup.open_questions)。
- **LOW** retry (最大 3 回・1 秒間隔) と全失敗時のユーザー再起動促しは、watcher 起動失敗を
  E2E から注入できないため実測不能。実装は `restart_watcher_with_retry` に存在し、
  全失敗時は log + S11 の restart-prompt に委譲する (spec.md#test-points の観測制約どおり)。
- **LOW** spec.md の upstream hash が domain docs 現行値と乖離。再 derive は C-DEC11 等を
  失うリスクがあるため保留 (status.yaml#followup.spec_drift)。

### Disposition

- 全 LOW 指摘は観測制約 / 既知の follow-up として status.yaml#followup に記録。
  実装の修正を要する指摘なし。

## Verdict

verdict=PASS (3 structural gates PASS + S22 E2E GREEN + 回帰 GREEN。
semantic reviewer は orchestrator 制約で skip — 詳細は #pass-1)
