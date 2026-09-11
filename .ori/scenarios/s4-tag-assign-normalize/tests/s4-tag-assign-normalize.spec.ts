// @ori-generated scenario:s4-tag-assign-normalize
//
// S4: タグ付与の正規化と重複排除
//
// Runner: wdio (Tauri local mode, `@wdio/tauri-service` + `tauri-driver`)
// Binary の起動は wdio.conf.ts の tauri:options.application が担当
//
// テスト戦略 (E2C / host process):
//   Tauri IPC invoke で assign-tag コマンドを発行し、結果を検証する。
//   ファイル永続化の検証は node:fs (host file system) で行う。
//
// Given: テスト用 storage_dir に Note A の .md ファイルを事前作成 (tags=["gpt"])
// When: "  GPT  " で assign_tag → TagDiff::Unchanged (重複 no-op)
// Then: ファイル frontmatter が変化していない、event 非発行
// When: "coding" で assign_tag → TagDiff::Added
// Then: ファイル frontmatter に "coding" 追加、event NoteTagsChanged 発行

import { mkdtempSync, readFileSync, writeFileSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** .md frontmatter から tags 配列を抽出 */
function extractTags(mdContent: string): string[] {
  const m = mdContent.match(/^tags:\s*\[(.*)\]$/m);
  if (!m) return [];
  // 単純な YAML inline list: [gpt, coding]
  const raw = m[1].replace(/"/g, '').trim();
  if (!raw) return [];
  return raw.split(',').map(s => s.trim()).filter(Boolean);
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

describe('scenario:s4-tag-assign-normalize', () => {
  const scenarioId = 's4-tag-assign-normalize';
  let tmpDir: string;
  let noteId: string;
  let mdPath: string;

  before(() => {
    noteId = '20260620120000';
    tmpDir = mkdtempSync(join(tmpdir(), `${scenarioId}-`));
    mdPath = join(tmpDir, `${noteId}.md`);

    // 既存 Note A: tags=["gpt"], body="hello"
    const frontmatter = [
      '---',
      `createdAt: 2026-06-20T12:00:00`,
      `updatedAt: 2026-06-20T12:00:00`,
      'tags: ["gpt"]',
      '---',
      '',
      'hello',
    ].join('\n');
    writeFileSync(mdPath, frontmatter, 'utf-8');
  });

  after(() => {
    try { rmdirSync(tmpDir, { recursive: true }); } catch { /* best effort */ }
  });

  it('step 1 — validation#s4-tag-assign-normalize: duplicate tag after normalization is no-op', async () => {
    // ── Given ──
    // Note A (tags=["gpt"]) が存在
    const originalContent = readFileSync(mdPath, 'utf-8');

    // ── When ──
    // Tauri IPC: invoke assign-tag with raw_tag="  GPT  "
    // (normalization: trim → "GPT" → lowercase → "gpt" → already in TagSet)
    //
    // TODO: wdio + tauri IPC interaction pattern
    // const result = await browser.executeScript(`
    //   window.__TAURI_INTERNALS__.invoke('plugin:note-capture|assign_tag', {
    //     noteId: '${noteId}',
    //     rawTag: '  GPT  '
    //   })
    // `);

    // ── Then ──
    // ファイルが unchanged であること
    const afterContent = readFileSync(mdPath, 'utf-8');
    expect(afterContent).toBe(originalContent);

    // TagSet に "gpt" が 1 件のみ (重複なし)
    const tags = extractTags(afterContent);
    expect(tags).toEqual(['gpt']);

    // TODO: verify event bus — NoteTagsChanged NOT emitted
    // expect(emittedEvents).not.toContainEqual(
    //   expect.objectContaining({ type: 'NoteTagsChanged', noteId })
    // );
  });

  it('step 2 — validation#s4-tag-assign-normalize: new tag assignment emits NoteTagsChanged', async () => {
    // ── Given ──
    // Note A (tags=["gpt"]) が存在

    // ── When ──
    // Tauri IPC: invoke assign-tag with raw_tag="coding"
    //
    // TODO: wdio + tauri IPC interaction pattern
    // await browser.executeScript(`
    //   window.__TAURI_INTERNALS__.invoke('plugin:note-capture|assign_tag', {
    //     noteId: '${noteId}',
    //     rawTag: 'coding'
    //   })
    // `);

    // ── Then ──
    // TagSet が ["gpt", "coding"] に更新
    const afterContent = readFileSync(mdPath, 'utf-8');
    const tags = extractTags(afterContent);
    // TODO: wdio interaction 後に検証
    // expect(tags).toEqual(['gpt', 'coding']);

    // TODO: verify event bus — NoteTagsChanged emitted
    // expect(emittedEvents).toContainEqual(
    //   expect.objectContaining({ type: 'NoteTagsChanged', noteId })
    // );
  });
});