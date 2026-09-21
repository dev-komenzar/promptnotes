// @ori-generated scenario:s17-external-file-modified-no-conflict
//
// 検証対象: validation.md#s17-external-file-modified-no-conflict
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A (20260620120000, body="hello") を seed する。app は起動時に
//        PageMain から start_file_watcher を invoke し、storage_dir を監視する。
//
// 核: テストプロセスが storage_dir の既存 .md を「外部変更」したとき、手動 Refresh / 再起動なしに
//     NoteFeed UI (既存 Block の body 表示) が自動で "hello" → "hello world" に更新されること。
//     これが watcher 検知 → 再 parse → NoteFileModifiedExternally → subscriber
//     (upsert_one + notes-changed) → frontend 再 hydrate の縦断 pipeline の間接観測になる
//     (domain event 自体は frontend に露出しない)。
//
// 注意: list_notes は毎回 disk から再 hydrate するため、テスト本体で手動 invoke して
//      「変更後 body が見える」ことを確認しても watcher の証明にはならない。よって step 1 / 3 は
//      list_notes を呼ばず DOM (CodeMirror .cm-content) の自動更新のみで判定する
//      (spec.md#impl-notes)。

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;

const NOTE_A = '20260620120000';
const BODY_INITIAL = 'hello';
const BODY_MODIFIED = 'hello world';
const BODY_MODIFIED_2 = 'hello world 2';

type NoteSummary = {
  id: string;
  body: string;
  tags: string[];
  created_at: string;
  updated_at: string;
};

type NoteFeed = { notes: NoteSummary[] };

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

async function invokeListNotes(): Promise<NoteFeed> {
  const result = await browser.tauri.execute(async (tauri) => {
    return await tauri.core.invoke('list_notes');
  });
  return result as NoteFeed;
}

describe('scenario:s17-external-file-modified-no-conflict', () => {
  before(async () => {
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({ timeout: 15000, timeoutMsg: 'seeded note A did not appear in feed' });
    await waitForBlockBody(NOTE_A, BODY_INITIAL);

    // Given: storage_dir には Note A のみ → フィード表示 1 件、body は "hello"
    expect(await blockIds()).toEqual([NOTE_A]);
  });

  it('step 1 — validation#s17-external-file-modified-no-conflict: 外部 .md 変更で既存 Block の body が自動更新', async () => {
    // When: 外部プログラムの代役として既存 .md の body を上書きする
    //       (list_notes の手動 invoke / 再起動は一切行わない)
    writeNoteMd(NOTE_A, BODY_MODIFIED);

    // Then: watcher 検知 → 再 parse → NoteFileModifiedExternally → notes-changed → 再 hydrate により
    //       既存 Block A の CM body が "hello world" に更新される
    await waitForBlockBody(NOTE_A, BODY_MODIFIED);

    // UI 通知は不要 (トーストが出ない)
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 2 — validation#s17-external-file-modified-no-conflict: 再 parse された Note の内容が正しい', async () => {
    // When: 反映後の read DTO を取得する
    const feed = await invokeListNotes();

    // Then: 再 parse された body / tags / 秒精度タイムスタンプが一致する
    //       (When は body のみ変更 → updatedAt は Given の t0 のまま)
    const noteA = feed.notes.find((n) => n.id === NOTE_A);
    expect(noteA).toBeDefined();
    expect(noteA?.body).toBe(BODY_MODIFIED);
    expect(noteA?.tags).toEqual([]);
    expect(noteA?.created_at).toBe('2026-06-20T12:00:00Z');
    expect(noteA?.updated_at).toBe('2026-06-20T12:00:00Z');
  });

  it('step 3 — validation#s17-external-file-modified-no-conflict: 連続する外部変更も反映され Block は増えない', async () => {
    // When: debounce 窓 (500ms) を超えて待ってから、さらに外部上書きする (手動操作なし)。
    //       watcher は同一ファイルへの 500ms 以内の連続イベントを 1 つに集約する設計
    //       (domain/workflows/detect-external-changes.md#notes) ため、
    //       2 回目の変更は debounce 窓を過ぎてから発行する。
    //       upsert_note が append でなく差し替えであることを見る。
    await browser.pause(700);
    writeNoteMd(NOTE_A, BODY_MODIFIED_2);

    // Then: DOM body が再び自動更新され、Block 数は 1 のまま (新規追加ではない)
    await waitForBlockBody(NOTE_A, BODY_MODIFIED_2);
    expect(await blockIds()).toEqual([NOTE_A]);
  });

  it('step 4 — validation#s17-external-file-modified-no-conflict: IDLE のため競合ダイアログも通知も出ない', async () => {
    // Then: Block A は IDLE 状態のまま (編集していない → S19 の競合ケースではない)
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    expect(await blockA.getAttribute('data-block-state')).toBe('IDLE');

    // Then: 競合ダイアログ・トーストいずれも表示されない
    expect(await $('[data-testid="widget-external-change-conflict"]').isExisting()).toBe(false);
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);

    // Then: 変更は保持されている (後続処理で巻き戻っていない)
    await waitForBlockBody(NOTE_A, BODY_MODIFIED_2);
  });
});