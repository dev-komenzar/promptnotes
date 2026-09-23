// @ori-generated scenario:s2-autosave-debounce
//
// 検証対象: validation.md#s2-autosave-debounce
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に Note A (20260620120000, body="hello") を seed する

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

function extractUpdatedAt(md: string): string | null {
  const m = md.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

describe('scenario:s2-autosave-debounce', () => {
  const NOTE_ID = '20260620120000';
  const MD_PATH = join(process.env.TAURI_TEST_STORAGE_DIR!, `${NOTE_ID}.md`);

  it('auto-save debounce: key input → 500ms wait → file updated', async () => {
    const before = readFileSync(MD_PATH, 'utf-8');
    expect(before).toContain('hello');

    const block = await $('[data-testid="screen-1-block"]');
    await block.click();
    await browser.waitUntil(
      async () => (await block.getAttribute('data-block-state')) === 'EDITING',
      { timeout: 5000, timeoutMsg: 'block did not enter EDITING' },
    );

    const editor = await block.$('.cm-editor .cm-content');
    await editor.waitForExist({ timeout: 3000 });
    await editor.click();

    const typedAt = Date.now();
    await browser.keys(' world');

    await browser.waitUntil(
      () => {
        try {
          return readFileSync(MD_PATH, 'utf-8').includes('hello world');
        } catch {
          return false;
        }
      },
      { timeout: 5000, interval: 50, timeoutMsg: 'autosave did not persist within 5s' },
    );
    const elapsedMs = Date.now() - typedAt;

    const after = readFileSync(MD_PATH, 'utf-8');
    expect(after).toContain('hello world');
    expect(extractUpdatedAt(after)).not.toBe(extractUpdatedAt(before));
    expect(elapsedMs).toBeGreaterThanOrEqual(400);
    expect(elapsedMs).toBeLessThanOrEqual(3000);
  });

  it('no-op autosave: body を元に戻す編集試行では永続化されない (S9 idempotency)', async () => {
    const block = await $('[data-testid="screen-1-block"]');
    await block.click();
    await browser.waitUntil(
      async () => (await block.getAttribute('data-block-state')) === 'EDITING',
      { timeout: 5000, timeoutMsg: 'block did not enter EDITING' },
    );
    const editor = await block.$('.cm-editor .cm-content');
    await editor.waitForExist({ timeout: 3000 });
    await editor.click();

    const before = readFileSync(MD_PATH, 'utf-8');
    const beforeUpdatedAt = extractUpdatedAt(before);

    await browser.keys(' ');
    await browser.keys(['Backspace']);
    await browser.pause(1200);

    const after = readFileSync(MD_PATH, 'utf-8');
    expect(after).toBe(before);
    expect(extractUpdatedAt(after)).toBe(beforeUpdatedAt);
  });
});
