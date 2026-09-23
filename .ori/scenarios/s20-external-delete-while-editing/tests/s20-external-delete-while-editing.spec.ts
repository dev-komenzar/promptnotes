// @ori-generated scenario:s20-external-delete-while-editing
//
// 検証対象: validation.md#s20-external-delete-while-editing
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A (20260620120000, body="hello") を seed する。app は起動時に
//        PageMain から start_file_watcher を invoke し、storage_dir を監視する。
//
// 核: ユーザが Block A を EDITING 中に、テストプロセスが storage_dir の既存 .md を
//     「外部削除」したとき、削除通知 (widget-external-delete-notice) が表示され、
//     Discard でフィードから除去され、Save as new file で元の id のまま .md が再作成され
//     Block が IDLE に戻ること。これが watcher 検知 → NoteFileDeletedExternally →
//     frontend の EDITING 検出 → 削除通知、の縦断 pipeline の間接観測になる
//     (domain event 自体は frontend に露出しない)。
//
// 注意: 日本語入力 (IME composition) は wdio から再現できないため、ローカル編集は ASCII
//      ("hello local") で表現する。意味 (body がディスクと乖離している) は同一 (spec.md#impl-notes)。

import { writeFileSync, unlinkSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;

const NOTE_A = '20260620120000';
const BODY_INITIAL = 'hello';
const BODY_LOCAL = 'hello local';

function noteMd(body: string, tags: string[] = []): string {
  return [
    '---',
    `createdAt: ${NOTE_A}`,
    `updatedAt: ${NOTE_A}`,
    `tags: [${tags.join(', ')}]`,
    '---',
    body,
  ].join('\n');
}

function writeNoteMd(body: string): void {
  writeFileSync(join(NOTES_DIR, `${NOTE_A}.md`), noteMd(body), 'utf-8');
}

function deleteNoteMd(): void {
  unlinkSync(join(NOTES_DIR, `${NOTE_A}.md`));
}

function readNoteMd(): string | null {
  const path = join(NOTES_DIR, `${NOTE_A}.md`);
  return existsSync(path) ? readFileSync(path, 'utf-8') : null;
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

async function waitForBlockGone(id: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    async () => !(await $(`[data-block-id="${id}"]`).isExisting()),
    { timeout: timeoutMs, interval: 200, timeoutMsg: `block ${id} did not disappear` }
  );
}

const DIALOG = '[data-testid="widget-external-delete-notice"]';

async function dialogExists(): Promise<boolean> {
  return await $(DIALOG).isExisting();
}

async function waitForDialog(timeoutMs = 15_000): Promise<void> {
  await browser.waitUntil(async () => await dialogExists(), {
    timeout: timeoutMs,
    interval: 200,
    timeoutMsg: 'delete notice widget-external-delete-notice did not appear',
  });
}

async function waitForDialogGone(timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(async () => !(await dialogExists()), {
    timeout: timeoutMs,
    interval: 200,
    timeoutMsg: 'delete notice did not close',
  });
}

async function waitForBlockState(id: string, expected: string, timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(async () => (await blockState(id)) === expected, {
    timeout: timeoutMs,
    interval: 100,
    timeoutMsg: `block ${id} state did not become ${expected}`,
  });
}

async function enterEditing(id: string): Promise<void> {
  const block = await $(`[data-block-id="${id}"]`);
  await block.click();
  await waitForBlockState(id, 'EDITING', 5_000);
  const editor = await block.$('.cm-editor .cm-content');
  await editor.waitForExist({ timeout: 3_000 });
  await editor.click();
}

// window blur (headless WebKitGTK の focus 管理) で EDITING → FOCUSED に落ちることがあるため、
// 通知を必要とする直前に EDITING を保証する。
async function ensureEditing(id: string): Promise<void> {
  if ((await blockState(id)) === 'EDITING') return;
  await enterEditing(id);
}

// The modal <dialog> lives in the browser top layer; the native WebDriver pointer click is flaky
// in this headless WebKitGTK window (window-focus management), so the notice controls are
// dispatched through a DOM click. The deletion trigger itself stays a real external file unlink.
async function clickDialogControl(testid: string): Promise<void> {
  const selector = `${DIALOG} [data-testid="${testid}"]`;
  await $(selector).waitForExist({ timeout: 5_000 });
  await browser.execute((sel) => {
    const target = document.querySelector(sel) as HTMLElement | null;
    if (!target) throw new Error(`dialog control not found: ${sel}`);
    target.click();
  }, selector);
}

describe('scenario:s20-external-delete-while-editing', () => {
  before(async () => {
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({
      timeout: 15_000,
      timeoutMsg: 'seeded note A did not appear in feed',
    });
    await waitForBlockBody(NOTE_A, BODY_INITIAL);

    // Given: storage_dir には Note A のみ → フィード表示 1 件、body "hello"、IDLE
    expect(await blockIds()).toEqual([NOTE_A]);
    expect(await blockState(NOTE_A)).toBe('IDLE');

    // Given: ユーザが EDITING 状態で body を編集する ("hello" → "hello local")。
    //        AutoSave debounce (500ms) の永続化を待ち、ディスク == ローカル body の状態にしてから
    //        外部削除を起こす (pending write が外部削除を race するのを避ける)。
    await enterEditing(NOTE_A);
    await browser.keys(' local');
    await browser.waitUntil(
      async () => (readNoteMd() ?? '').includes(BODY_LOCAL),
      { timeout: 5000, interval: 100, timeoutMsg: 'local autosave did not persist within 5s' }
    );
    await browser.pause(1200);
    await ensureEditing(NOTE_A);
    expect(await blockState(NOTE_A)).toBe('EDITING');
    await waitForBlockBody(NOTE_A, BODY_LOCAL);
  });

  it('step 1 — validation#s20-external-delete-while-editing: EDITING 中の外部削除で削除通知が表示される', async () => {
    // When: 外部プログラムの代役として既存 .md を削除する
    //       (list_notes の手動 invoke / 再起動は一切行わない)
    await ensureEditing(NOTE_A);
    deleteNoteMd();

    // Then: watcher 検知 → NoteFileDeletedExternally → frontend の EDITING 検出 →
    //       削除通知が表示され、Block A は EDITING のまま (黙って消えない)
    await waitForDialog();
    expect(await blockState(NOTE_A)).toBe('EDITING');

    // 削除通知の過程で toast は出ない (screen-1 Notes)
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 2 — validation#s20-external-delete-while-editing: 通知が編集中 body を提示する', async () => {
    // Then: note title と現在の編集中 body が表示される
    const title = await $(`${DIALOG} [data-testid="delete-notice-title"]`);
    expect((await title.getText()).trim()).toBe(`${NOTE_A}.md`);

    const localView = await $(`${DIALOG} [data-testid="delete-notice-body-local"]`);
    expect(await localView.getValue()).toBe(BODY_LOCAL);
  });

  it('step 3 — validation#s20-external-delete-while-editing: Save as new file で元の id のまま .md を再作成し IDLE へ', async () => {
    // When: 「新規ファイルとして保存」を選ぶ
    await clickDialogControl('delete-notice-save-as-new');

    // Then: 通知が閉じ、storage_dir/<A.id>.md が編集中内容で再作成され、Block A は IDLE に遷移する
    await waitForDialogGone();
    await waitForBlockState(NOTE_A, 'IDLE');
    const restored = readNoteMd();
    expect(restored).not.toBeNull();
    expect(restored).toContain(BODY_LOCAL);
    await waitForBlockBody(NOTE_A, BODY_LOCAL);
  });

  it('step 4 — validation#s20-external-delete-while-editing: Discard でフィードから除去される', async () => {
    // When: 再度 EDITING に入れ、外部削除でもう一度通知を表示させ、「破棄」を選ぶ
    await enterEditing(NOTE_A);
    // debounce 窓 (500ms) を超えて待つ。直前の Save as new file の自書き込み (Created) と
    // 同一パスで窓内だと削除イベントが破棄されるため (s18 step 3 と同じ理由)。
    await browser.pause(700);
    await ensureEditing(NOTE_A);
    deleteNoteMd();
    await waitForDialog();
    await clickDialogControl('delete-notice-discard');

    // Then: 通知が閉じ、フィードから Block A が消える (remove_note / I-F8)
    await waitForDialogGone();
    await waitForBlockGone(NOTE_A);
    expect(await blockIds()).toEqual([]);
    expect(await $('[data-testid="screen-1-feed-empty"]').isExisting()).toBe(true);
  });

  it('step 5 — validation#s20-external-delete-while-editing: IDLE 中の外部削除では通知を出さない', async () => {
    // When: Note A を外部作成して IDLE に戻し、その状態で外部削除する (S18 のケース)
    writeNoteMd(BODY_INITIAL);
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({ timeout: 10_000, timeoutMsg: 'note A did not reappear after recreate' });
    await waitForBlockState(NOTE_A, 'IDLE');
    // 直前の外部作成 (Created) と窓内で破棄されないよう debounce 窓を超えて待つ。
    await browser.pause(700);

    deleteNoteMd();

    // Then: 削除通知は表示されず、フィードから Block A が消える (S18 と同じ silent)
    await waitForBlockGone(NOTE_A);
    expect(await dialogExists()).toBe(false);
    expect(await $(`[data-block-id="${NOTE_A}"]`).isExisting()).toBe(false);
  });
});
