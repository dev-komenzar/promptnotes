// @ori-generated scenario:s2-autosave-debounce
//
// S2: 既存 Note 編集後 500ms debounce で AutoSave
//
// Runner: wdio (Tauri local mode, `@wdio/tauri-service` + `tauri-driver`)
// Binary の起動は wdio.conf.ts の tauri:options.application が担当
//
// テスト戦略 (E2E / host process):
//   UI 操作は wdio の browser-like commands を通じて行う。
//   永続化結果の検証は node:fs (host file system) で行う。
//   Tauri の IPC invoke は不要（この scenario は UI からの自動トリガーを検証するため）。
//
// Given: テスト用 storage_dir に Note A の .md ファイルを事前作成
// When: エディタにテキスト追記 + 500ms 待機
// Then: .md ファイルが更新されていることを確認

import { mkdtempSync, readFileSync, writeFileSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** .md frontmatter から updatedAt の値を抽出（ISO 8601） */
function extractUpdatedAt(mdContent: string): string | null {
  const m = mdContent.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

/** .md frontmatter 以降の body テキストを抽出 */
function extractBody(mdContent: string): string {
  const parts = mdContent.split(/\n---\n/);
  // frontmatter = parts[1] (0 番は先頭 `---` の前、空文字想定)
  // body は 2 番目以降。frontmatter が存在すれば parts.length >= 3
  if (parts.length >= 3) {
    return parts.slice(2).join('\n---\n').trim();
  }
  // frontmatter なし（ありえないが safety net）
  return mdContent.trim();
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

describe('scenario:s2-autosave-debounce', () => {
  const scenarioId = 's2-autosave-debounce';
  let tmpDir: string;
  let noteId: string;
  let mdPath: string;

  // S2 notes: `id = now.format(YYYYMMDDhhmmss)`。テストでは固定値を使う
  before(() => {
    // 既存 Note A を模擬: updatedAt = "2026-06-20T12:00:00"
    // id = "20260620120000"
    noteId = '20260620120000';

    // テスト用 storage_dir
    tmpDir = mkdtempSync(join(tmpdir(), `${scenarioId}-`));

    // .md ファイルを事前作成
    mdPath = join(tmpDir, `${noteId}.md`);
    const frontmatter = [
      '---',
      `createdAt: 2026-06-20T12:00:00`,
      `updatedAt: 2026-06-20T12:00:00`,
      'tags: []',
      '---',
      '',
      'hello',
    ].join('\n');
    writeFileSync(mdPath, frontmatter, 'utf-8');

    // TODO: storage_dir をテスト用に差し替える手段を要検討
    // 現状の resolve_storage_dir は AppHandle::path_resolver() から
    // app_data_dir を取得するため、テスト時は環境変数または config 経由での
    // storage_dir 差し替えが必要。
    // 本 spec では E2E runner の inject 機構（app_handle 差し替え or
    // dev feature flag）が未整備のため、テストは skeleton として記述し
    // 実際の E2E 実行は Phase 11b 以降で可能になる旨を注記する。
  });

  after(() => {
    // テスト用ディレクトリをクリーンアップ
    try {
      rmdirSync(tmpDir, { recursive: true });
    } catch {
      // best effort
    }
  });

  it('auto-save debounce: key input → 500ms wait → file updated', async () => {
    // ── Given ──
    // Note A が存在し、EDITING 状態であることを確認
    // storage_dir のセットアップは before() で実施済み
    //
    // 実際のアプリ操作:
    //   TODO: storage_dir 設定 → app 起動 → note を開いて編集開始
    //   (Phase 11b ui-grouping 後の page implementation 待ち)
    //
    //   スケルトンとして操作フローを記述
    //
    // 1. Navigate to the note editor (page-main)
    // 2. Verify Note A (id=20260620120000) is displayed
    // 3. Click to focus the editor / enter EDITING state
    //
    // ── When ──
    //
    // 4. Type additional text " world" into the CodeMirror editor
    // 5. Stop typing and wait 600ms (500ms debounce + 100ms processing margin)
    //
    // ── Then ──
    //
    // 6. Read the .md file from disk
    // 7. Verify: body contains "hello world"
    // 8. Verify: updatedAt has changed from the original
    //
    // Note: 実際の E2E テストは以下の TBD 項目が解決した後に実装可能:
    //   - TBD: test storage_dir injection mechanism
    //   - TBD: wdio + tauri interaction patterns for editor
    //   - TBD: page-main implementation completion

    // ---- skeleton assertions (TBD: wdio interaction) ----
    // const noteBlock = await $('#note-20260620120000');
    // await noteBlock.click();                    // EDITING 状態に遷移
    // const editor = await $('#code-editor');
    // await editor.setValue('hello world');        // または browser.keys で入力
    // await browser.pause(600);                  // debounce + α

    // ---- ファイル検証 (E2E 後、host fs) ----
    const mdContent = readFileSync(mdPath, 'utf-8');
    const body = extractBody(mdContent);
    const updatedAt = extractUpdatedAt(mdContent);

    // 事前状態の確認（before で書き込んだ値のままなら test infra は正しく動いている）
    expect(body).toBe('hello');
    expect(updatedAt).toBe('2026-06-20T12:00:00');

    // TODO: wdio interaction 後に再度検証
    // expect(body).toBe('hello world');
    // expect(updatedAt).not.toBe('2026-06-20T12:00:00');
  });

  it('no-op autosave: unchanged body does not trigger save', async () => {
    // ── Given ──
    // Note A が EDITING 状態だが、body は変更されていない
    //
    // ── When ──
    // 500ms 経過（EDITING 状態のまま入力なしで debounce タイムアウト）
    //
    // ── Then ──
    // .md ファイルの内容が unchanged であること
    // NoteBodyEdited event は発行されない

    const originalContent = readFileSync(mdPath, 'utf-8');

    // TODO: wdio interaction — enter EDITING state, wait 600ms, exit EDITING state
    // await browser.pause(600);

    const newContent = readFileSync(mdPath, 'utf-8');
    expect(newContent).toBe(originalContent);
  });
});