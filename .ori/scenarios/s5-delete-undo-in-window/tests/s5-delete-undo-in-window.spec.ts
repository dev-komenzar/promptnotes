// @ori-generated scenario:s5-delete-undo-in-window
//
// 検証対象: validation.md#s5-delete-undo-in-window
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に Note A (20260620120000) を seed する

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

function notePath(storageDir: string, id: string): string {
  return join(storageDir, `${id}.md`);
}

async function count(selector: string): Promise<number> {
  const els = await $$(selector);
  return (els as unknown as { length: number }).length;
}

async function dispatchClick(testid: string): Promise<void> {
  await browser.execute((id: string) => {
    const btn = document.querySelector<HTMLElement>(`[data-testid="${id}"]`);
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  }, testid);
}

describe('scenario:s5-delete-undo-in-window', () => {
  const STORAGE_DIR = process.env.TAURI_TEST_STORAGE_DIR!;
  const NOTE_ID = '20260620120000';
  const NOTE_BODY = 'hello';
  const NOTE_PATH = notePath(STORAGE_DIR, NOTE_ID);

  before(async () => {
    await browser.waitUntil(async () => (await count('[data-testid="screen-1-block"]')) >= 1, {
      timeout: 10000,
      timeoutMsg: 'seeded note did not appear in feed',
    });
    await browser.pause(1500);
  });

  it('deletes a note (moves to OS trash) then undoes restore within the toast window', async () => {
    // ---- Step 1: confirm initial state ----
    const countBefore = await count('[data-testid="screen-1-block"]');
    expect(countBefore).toBeGreaterThanOrEqual(1);
    expect(existsSync(NOTE_PATH)).toBe(true);

    // ---- Step 2: delete the note ----
    // delete ボタンは hover 時のみ操作可能（opacity-0 / pointer-events-none）。
    // WebKitWebDriver の moveTo は CSS :hover を安定して発火しないため、
    // 対象 button に click イベントを直接 dispatch する。
    await dispatchClick('screen-1-block-delete');
    await browser.pause(1000);

    expect(existsSync(NOTE_PATH)).toBe(false);
    const countAfterDelete = await count('[data-testid="screen-1-block"]');
    expect(countAfterDelete).toBe(countBefore - 1);
    const toastCount = await count('[data-testid="screen-1-toast"]');
    expect(toastCount).toBeGreaterThanOrEqual(1);

    // ---- Step 3: undo the delete ----
    await dispatchClick('screen-1-toast-undo');
    await browser.pause(1000);

    expect(existsSync(NOTE_PATH)).toBe(true);
    const restored = readFileSync(NOTE_PATH, 'utf-8');
    expect(restored).toContain(NOTE_BODY);
    const countAfterUndo = await count('[data-testid="screen-1-block"]');
    expect(countAfterUndo).toBe(countBefore);
    const remainingToasts = await count('[data-testid="screen-1-toast"]');
    expect(remainingToasts).toBe(0);
  });
});
