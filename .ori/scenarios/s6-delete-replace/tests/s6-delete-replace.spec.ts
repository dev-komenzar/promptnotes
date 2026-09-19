// @ori-generated scenario:s6-delete-replace
//
// 検証対象: validation.md#s6-delete-replace
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に
//        Note A (20260620120000, body=alpha) / Note B (20260620130000, body=bravo) を seed する

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

function notePath(storageDir: string, id: string): string {
  return join(storageDir, `${id}.md`);
}

async function count(selector: string): Promise<number> {
  const els = await $$(selector);
  return (els as unknown as { length: number }).length;
}

async function dispatchClick(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    const el = document.querySelector<HTMLElement>(sel);
    el?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  }, selector);
}

/** トーストの画面上の Y 座標 (top)。存在しなければ Infinity。 */
async function toastTop(id: string): Promise<number> {
  const top = await browser.execute((noteId: string) => {
    const el = document.querySelector<HTMLElement>(
      `[data-testid="screen-1-toast"][data-toast-id="${noteId}"]`
    );
    return el ? el.getBoundingClientRect().top : Number.POSITIVE_INFINITY;
  }, id);
  return typeof top === 'number' ? top : Number.POSITIVE_INFINITY;
}

describe('scenario:s6-delete-replace', () => {
  const STORAGE_DIR = process.env.TAURI_TEST_STORAGE_DIR!;
  const A = '20260620120000';
  const B = '20260620130000';
  const PATH_A = notePath(STORAGE_DIR, A);
  const PATH_B = notePath(STORAGE_DIR, B);

  const deleteBtn = (id: string) =>
    `[data-block-id="${id}"] [data-testid="screen-1-block-delete"]`;
  const undoBtn = (id: string) =>
    `[data-testid="screen-1-toast-undo"][data-toast-id="${id}"]`;
  const toast = (id: string) =>
    `[data-testid="screen-1-toast"][data-toast-id="${id}"]`;

  before(async () => {
    await browser.waitUntil(async () => (await count('[data-testid="screen-1-block"]')) >= 2, {
      timeout: 10000,
      timeoutMsg: 'seeded notes A/B did not appear in feed',
    });
    await browser.pause(1500);
  });

  it('stacks toasts on consecutive deletes and undoes each note independently', async () => {
    // ---- Given: A と B が表示され、どちらも未削除 ----
    const countBefore = await count('[data-testid="screen-1-block"]');
    expect(countBefore).toBe(2);
    expect(existsSync(PATH_A)).toBe(true);
    expect(existsSync(PATH_B)).toBe(true);
    expect(await count('[data-testid="screen-1-toast"]')).toBe(0);

    // ---- When 1 (t0): A を削除 ----
    await dispatchClick(deleteBtn(A));
    await browser.pause(1000);

    // ---- Then (t0) ----
    expect(existsSync(PATH_A)).toBe(false);
    expect(await count('[data-testid="screen-1-block"]')).toBe(1);
    expect(await count('[data-testid="screen-1-toast"]')).toBe(1);
    expect(await count(toast(A))).toBe(1);

    // ---- When 2 (t1): A のトースト表示中に B を削除 ----
    await dispatchClick(deleteBtn(B));
    await browser.pause(1000);

    // ---- Then (t1): A は置換されず、B が上に積まれる ----
    expect(existsSync(PATH_B)).toBe(false);
    expect(await count('[data-testid="screen-1-block"]')).toBe(0);
    expect(await count('[data-testid="screen-1-toast"]')).toBe(2);
    expect(await count(toast(A))).toBe(1);
    expect(await count(toast(B))).toBe(1);

    // 積み上げ順: 最新 (B) が画面上側 (Y 座標が小さい) にあること
    const topA = await toastTop(A);
    const topB = await toastTop(B);
    expect(topA).toBeLessThan(Number.POSITIVE_INFINITY);
    expect(topB).toBeLessThan(Number.POSITIVE_INFINITY);
    expect(topB).toBeLessThan(topA);

    // ---- When 3 (t2): A の Undo ----
    await dispatchClick(undoBtn(A));
    await browser.pause(1000);

    // ---- Then (t2): A のみ復元。B のトーストは表示維持 ----
    expect(existsSync(PATH_A)).toBe(true);
    expect(readFileSync(PATH_A, 'utf-8')).toContain('alpha');
    expect(existsSync(PATH_B)).toBe(false);
    expect(await count(toast(A))).toBe(0);
    expect(await count(toast(B))).toBe(1);
    expect(await count('[data-testid="screen-1-block"]')).toBe(1);

    // ---- When 4 (t3): B の Undo ----
    await dispatchClick(undoBtn(B));
    await browser.pause(1000);

    // ---- Then (t3): B も復元され、スタックが空になる ----
    expect(existsSync(PATH_B)).toBe(true);
    expect(readFileSync(PATH_B, 'utf-8')).toContain('bravo');
    expect(await count('[data-testid="screen-1-toast"]')).toBe(0);
    expect(await count('[data-testid="screen-1-block"]')).toBe(2);
  });
});
