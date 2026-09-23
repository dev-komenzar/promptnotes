// @ori-generated scenario:s19-external-modify-while-editing
//
// 検証対象: validation.md#s19-external-modify-while-editing
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A (20260620120000, body="hello") を seed する。app は起動時に
//        PageMain から start_file_watcher を invoke し、storage_dir を監視する。
//
// 核: ユーザが Block A を EDITING 中に、テストプロセスが storage_dir の既存 .md を
//     「外部変更」したとき、競合ダイアログ (widget-external-change-conflict, screen-4) が表示され、
//     KeepEditing で編集中の内容が保持され、ApplyExternal で外部変更が適用され IDLE に戻ること。
//     これが watcher 検知 → NoteFileModifiedExternally → frontend 競合判定 (EDITING + is_stale) →
//     screen-4 表示、の縦断 pipeline の間接観測になる (domain event 自体は frontend に露出しない)。
//
// 注意: 日本語入力 (IME composition) は wdio から再現できないため、ローカル編集は ASCII
//      ("hello local") で表現する。意味 (body がディスクと乖離している) は同一 (spec.md#impl-notes)。

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;

const NOTE_A = '20260620120000';
const BODY_INITIAL = 'hello';
const BODY_LOCAL = 'hello local';
const BODY_EXTERNAL = 'hello world';
const BODY_EXTERNAL_2 = 'hello world 2';
const BODY_EXTERNAL_3 = 'hello world 3';

function writeNoteMd(id: string, body: string, tags: string[] = []): void {
  const content = [
    '---',
    `createdAt: ${id}`,
    `updatedAt: ${id}`,
    `tags: [${tags.join(', ')}]`,
    '---',
    body,
  ].join('\n');
  writeFileSync(join(NOTES_DIR, `${id}.md`), content, 'utf-8');
}

async function blockIds(): Promise<string[]> {
  const blocks = await $$('[data-block-id]');
  const ids: string[] = [];
  for (const block of blocks) {
    const id = await block.getAttribute('data-block-id');
    if (id !== null) ids.push(id);
  }
  return ids;
}

async function blockState(id: string): Promise<string | null> {
  return await $(`[data-block-id="${id}"]`).getAttribute('data-block-state');
}

async function blockBodyText(id: string): Promise<string> {
  const content = await $(`[data-block-id="${id}"] .cm-content`);
  if (!(await content.isExisting())) return '';
  return (await content.getText()).trim();
}

async function waitForBlockBody(id: string, expected: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(async () => (await blockBodyText(id)) === expected, {
    timeout: timeoutMs,
    interval: 200,
    timeoutMsg: `block ${id} body did not become "${expected}"`,
  });
}

const DIALOG = '[data-testid="widget-external-change-conflict"]';

async function dialogExists(): Promise<boolean> {
  return await $(DIALOG).isExisting();
}

async function waitForDialog(timeoutMs = 15_000): Promise<void> {
  await browser.waitUntil(async () => await dialogExists(), {
    timeout: timeoutMs,
    interval: 200,
    timeoutMsg: 'conflict dialog widget-external-change-conflict did not appear',
  });
}

async function waitForDialogGone(timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(async () => !(await dialogExists()), {
    timeout: timeoutMs,
    interval: 200,
    timeoutMsg: 'conflict dialog did not close',
  });
}

async function enterEditing(id: string): Promise<void> {
  const block = await $(`[data-block-id="${id}"]`);
  await block.click();
  await browser.waitUntil(async () => (await blockState(id)) === 'EDITING', {
    timeout: 5_000,
    interval: 100,
    timeoutMsg: `block ${id} did not enter EDITING`,
  });
  const editor = await block.$('.cm-editor .cm-content');
  await editor.waitForExist({ timeout: 3_000 });
  await editor.click();
}

// The modal <dialog> lives in the browser top layer; the native WebDriver pointer click is flaky
// in this headless WebKitGTK window (window-focus management), so the resolution controls are
// dispatched through a DOM click. The conflict trigger itself stays a real external file write.
async function clickDialogControl(testid: string): Promise<void> {
  const selector = `${DIALOG} [data-testid="${testid}"]`;
  await $(selector).waitForExist({ timeout: 5_000 });
  await browser.execute((sel) => {
    const target = document.querySelector(sel) as HTMLElement | null;
    if (!target) throw new Error(`dialog control not found: ${sel}`);
    target.click();
  }, selector);
}

describe('scenario:s19-external-modify-while-editing', () => {
  before(async () => {
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({
      timeout: 15000,
      timeoutMsg: 'seeded note A did not appear in feed',
    });
    await waitForBlockBody(NOTE_A, BODY_INITIAL);

    // Given: storage_dir には Note A のみ → フィード表示 1 件、body "hello"、IDLE
    expect(await blockIds()).toEqual([NOTE_A]);
    expect(await blockState(NOTE_A)).toBe('IDLE');

    // Given: ユーザが EDITING 状態で body を編集する ("hello" → "hello local")。
    //        AutoSave debounce (500ms) の永続化を待ち、ディスク == ローカル body の状態にしてから
    //        外部変更を起こす (pending write が外部変更を上書きする race を避ける)。
    await enterEditing(NOTE_A);
    await browser.keys(' local');
    await browser.waitUntil(
      async () => {
        try {
          const { readFileSync } = await import('node:fs');
          return readFileSync(join(NOTES_DIR, `${NOTE_A}.md`), 'utf-8').includes(BODY_LOCAL);
        } catch {
          return false;
        }
      },
      { timeout: 5000, interval: 100, timeoutMsg: 'local autosave did not persist within 5s' },
    );
    await browser.pause(1200);
    expect(await blockState(NOTE_A)).toBe('EDITING');
    await waitForBlockBody(NOTE_A, BODY_LOCAL);
  });

  it('step 1 — validation#s19-external-modify-while-editing: EDITING 中の外部変更で競合ダイアログが表示される', async () => {
    // When: 外部プログラムの代役として既存 .md の body を上書きする
    //       (list_notes の手動 invoke / 再起動は一切行わない)
    writeNoteMd(NOTE_A, BODY_EXTERNAL);

    // Then: watcher 検知 → NoteFileModifiedExternally → frontend 競合判定 (EDITING + is_stale) →
    //       screen-4 ダイアログが表示され、Block A は EDITING のまま
    await waitForDialog();
    expect(await blockState(NOTE_A)).toBe('EDITING');

    // 競合解消の過程で toast は出ない (screen-4 Notes)
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 2 — validation#s19-external-modify-while-editing: ダイアログが local / external 両バージョンを提示する', async () => {
    // Then: note title / 編集中の内容 / 外部変更の内容 が compare-layout に表示される
    const title = await $(`${DIALOG} [data-testid="screen-4-note-title"]`);
    expect((await title.getText()).trim()).toBe(`${NOTE_A}.md`);

    const localView = await $(`${DIALOG} [data-testid="screen-4-body-local"]`);
    const externalView = await $(`${DIALOG} [data-testid="screen-4-body-external"]`);
    expect(await localView.getValue()).toBe(BODY_LOCAL);
    expect(await externalView.getValue()).toBe(BODY_EXTERNAL);
  });

  it('step 3 — validation#s19-external-modify-while-editing: KeepEditing (Cancel) で編集中の内容を保持する', async () => {
    // When: Cancel (KeepEditing と等価) を選択する
    await clickDialogControl('screen-4-cancel');

    // Then: ダイアログが閉じ、編集中の内容は外部版に置換されず保持され、Block は EDITING のまま
    await waitForDialogGone();
    expect(await blockState(NOTE_A)).toBe('EDITING');
    await waitForBlockBody(NOTE_A, BODY_LOCAL);
  });

  it('step 4 — validation#s19-external-modify-while-editing: ApplyExternal で外部変更を適用し IDLE へ遷移する', async () => {
    // When: さらに外部変更を起こしてダイアログを再表示させ、「外部変更を適用」を選んで確定する
    writeNoteMd(NOTE_A, BODY_EXTERNAL_2);
    await waitForDialog();
    await clickDialogControl('screen-4-resolution-apply-external');
    await clickDialogControl('screen-4-confirm');

    // Then: ダイアログが閉じ、Editor が外部版で置換され、Block は IDLE に遷移する
    await waitForDialogGone();
    await browser.waitUntil(async () => (await blockState(NOTE_A)) === 'IDLE', {
      timeout: 5_000,
      interval: 100,
      timeoutMsg: 'block A did not return to IDLE after ApplyExternal',
    });
    await waitForBlockBody(NOTE_A, BODY_EXTERNAL_2);
  });

  it('step 5 — validation#s19-external-modify-while-editing: IDLE 中の外部変更ではダイアログを出さずフィードを自動更新する', async () => {
    // When: Block A が IDLE の状態で外部変更を起こす (S17 のケース)
    writeNoteMd(NOTE_A, BODY_EXTERNAL_3);

    // Then: 競合ダイアログは表示されず、フィードの body が自動更新される (I-WC2 silent / I-F8)
    await waitForBlockBody(NOTE_A, BODY_EXTERNAL_3);
    expect(await dialogExists()).toBe(false);
    expect(await blockState(NOTE_A)).toBe('IDLE');
  });
});
