// @ori-generated scenario:s5-delete-undo-in-window
//
// 検証対象: validation.md#s5-delete-undo-in-window
// runner: wdio (tauri) — compose-service 参加者なし
//
// NOTE: WDIO の describe/it/expect/browser は Mocha グローバル注入。
// 型チェックを pass させるため tsconfig.json で `@wdio/globals/types` を読み込む。

import { existsSync, readFileSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';

// ---- helpers ----

/** storageDir から note ファイルの絶対パスを組み立てる */
function notePath(storageDir: string, id: string): string {
  return join(storageDir, `${id}.md`);
}

/**
 * アプリ側（WDIO 経由）で note を作成し、Feed に表示されるまで待つ。
 * 実装詳細: Cmd+N → body 入力 → Cmd+Enter → Feed に block が現れたら戻る。
 */
async function createNoteViaUI(body: string): Promise<void> {
  await browser.keys(['Meta', 'n']); // Cmd+N
  await browser.keys(body.split(''));
  await browser.keys(['Meta', 'Enter']);
  await browser.waitUntil(
    async () => (await browser.$$('[data-testid="note-block"]')).length > 0,
    { timeout: 5000, timeoutMsg: 'note block did not appear in feed' },
  );
}

/**
 * フィード内の全 note block の数で visible 判定。
 */
async function getNoteCount(): Promise<number> {
  const blocks = await browser.$$('[data-testid="note-block"]');
  return blocks.length;
}

// ---- test suite ----

describe('scenario:s5-delete-undo-in-window', () => {
  const STORAGE_DIR = process.env.STORAGE_DIR!;
  const NOTE_ID = '20260620120000';
  const NOTE_BODY = 'hello';
  const NOTE_PATH = notePath(STORAGE_DIR, NOTE_ID);

  before(async () => {
    // Note A が Feed に存在する状態を作る
    await createNoteViaUI(NOTE_BODY);
  });

  it('deletes a note (moves to OS trash) then undoes restore within the toast window', async () => {
    // ---- Step 1: confirm initial state ----
    const countBefore = await getNoteCount();
    expect(countBefore).toBeGreaterThanOrEqual(1);
    expect(existsSync(NOTE_PATH)).toBe(true);

    // ---- Step 2: delete the note ----
    // hover → delete button click
    const noteBlock = await $('[data-testid="note-block"]');
    await noteBlock.moveTo();
    const deleteBtn = await noteBlock.$('[data-testid="delete-btn"]');
    await deleteBtn.click();

    // Wait for deletion to propagate (animation + file ops)
    await browser.pause(1000);

    // Verify: file is gone (moved to OS trash)
    expect(existsSync(NOTE_PATH)).toBe(false);

    // Verify: note removed from Feed
    const countAfterDelete = await getNoteCount();
    expect(countAfterDelete).toBe(countBefore - 1);

    // Verify: undo toast is visible
    const toast = await browser.$('[data-testid="undo-toast"]');
    expect(await toast.isDisplayed()).toBe(true);

    // ---- Step 3: undo the delete ----
    const undoBtn = await toast.$('[data-testid="undo-btn"]');
    await undoBtn.click();

    // Wait for restore to propagate
    await browser.pause(1000);

    // Verify: file is restored
    expect(existsSync(NOTE_PATH)).toBe(true);
    const restored = readFileSync(NOTE_PATH, 'utf-8');
    expect(restored).toContain(NOTE_BODY);

    // Verify: note reappears in Feed
    const countAfterUndo = await getNoteCount();
    expect(countAfterUndo).toBe(countBefore);

    // Verify: toast is closed
    expect(await toast.isDisplayed()).toBe(false);
  });
});