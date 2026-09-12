// @ori-generated scenario:s3-flush-on-blur
//
// Scenario: フォーカス喪失で即時 Flush（debounce 待たず）
//
// domain/validation.md#s3-flush-on-blur:
//   - GIVEN: 既存 Note A, EDITING 状態。t1 に編集（AutoSave debounce timer は 500ms 待ち中）
//   - WHEN:  t1+0.2s=t2 時点でユーザが別ブロックをクリック。Block A は EDITING → IDLE 遷移、フォーカス喪失
//   - THEN:  debounce timer キャンセル。即時 Note::edit_body(now=t2) を実行（Flush）。NoteBodyEdited 発行
//
// runner: wdio (Tauri desktop app — .ori/architecture.md workspace.apps[].runtime.mode=local, runner=wdio)

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTE_A = '20260620120000';
const NOTE_B = '20260620130000';

function readNote(id: string): string {
  return readFileSync(join(process.env.TAURI_TEST_STORAGE_DIR!, `${id}.md`), 'utf-8');
}

function extractUpdatedAt(md: string): string | null {
  const m = md.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

function block(id: string) {
  return $(`[data-block-id="${id}"]`);
}

async function stateOf(id: string): Promise<string | null> {
  return (await block(id)).getAttribute('data-block-state');
}

async function enterEditing(id: string): Promise<void> {
  const el = await block(id);
  await el.click();
  await browser.waitUntil(
    async () => (await el.getAttribute('data-block-state')) === 'EDITING',
    { timeout: 5000, timeoutMsg: `block ${id} did not enter EDITING` },
  );
}

async function focusEditor(id: string): Promise<void> {
  const el = await block(id);
  const editor = await el.$('.cm-editor .cm-content');
  await editor.waitForExist({ timeout: 3000 });
  await editor.click();
}

describe('scenario:s3-flush-on-blur', () => {
  before(async () => {
    await (await block(NOTE_A)).waitForExist({ timeout: 10000 });
    await (await block(NOTE_B)).waitForExist({ timeout: 10000 });
    await browser.pause(1500);
  });

  it('step 1 — validation#s3-flush-on-blur: 別ブロッククリックで debounce を待たず即時 Flush し、重複保存しない', async () => {
    const before = readNote(NOTE_A);
    expect(before).toContain('hello A');
    const beforeUpdatedAt = extractUpdatedAt(before);

    // blur 先の要素は事前解決しておく (timed region に findElement 往復を入れない)
    const elB = await block(NOTE_B);

    await enterEditing(NOTE_A);
    await focusEditor(NOTE_A);

    const editedAt = Date.now();
    await browser.keys('!');
    await browser.pause(50);
    await elB.click();

    await browser.waitUntil(
      () => {
        try {
          return readNote(NOTE_A) !== before;
        } catch {
          return false;
        }
      },
      { timeout: 3000, interval: 30, timeoutMsg: 'flush did not persist after blur' },
    );

    const after = readNote(NOTE_A);
    expect(after).toContain('hello A!');
    expect(extractUpdatedAt(after)).not.toBe(beforeUpdatedAt);
    // debounce timer (500ms) より前に flush が完了している
    expect(Date.now() - editedAt).toBeLessThan(500);

    // debounce timer がキャンセルされ、後から 2 度目の書き込みが起きない
    const updatedAtAfterFlush = extractUpdatedAt(after);
    await browser.pause(800);
    expect(extractUpdatedAt(readNote(NOTE_A))).toBe(updatedAtAfterFlush);
  });

  it('step 2 — validation#s3-flush-on-blur: body が変化していなければ Flush は no-op', async () => {
    const before = readNote(NOTE_B);
    const beforeUpdatedAt = extractUpdatedAt(before);

    await enterEditing(NOTE_B);
    await focusEditor(NOTE_B);
    await browser.keys('!');
    await browser.keys(['Backspace']);
    await browser.pause(150);

    await (await block(NOTE_A)).click();
    await browser.pause(900);

    const after = readNote(NOTE_B);
    expect(after).toBe(before);
    expect(extractUpdatedAt(after)).toBe(beforeUpdatedAt);
  });

  it('step 3 — I-PM10: 同時に EDITING なブロックは高々 1 つ', async () => {
    await enterEditing(NOTE_A);
    expect(await stateOf(NOTE_A)).toBe('EDITING');

    await enterEditing(NOTE_B);
    await browser.waitUntil(async () => (await stateOf(NOTE_A)) === 'IDLE', {
      timeout: 5000,
      timeoutMsg: 'Block A did not return to IDLE after Block B became EDITING',
    });
    expect(await stateOf(NOTE_A)).toBe('IDLE');
    expect(await stateOf(NOTE_B)).toBe('EDITING');
  });
});
