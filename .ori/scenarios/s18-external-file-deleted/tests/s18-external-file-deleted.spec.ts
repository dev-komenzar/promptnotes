// @ori-generated scenario:s18-external-file-deleted
//
// 検証対象: validation.md#s18-external-file-deleted
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A (20260620120000, body="hello") と Note B (20260621090000, body="world") を seed する。
//        app は起動時に PageMain から start_file_watcher を invoke し、storage_dir を監視する。
//
// 核: テストプロセスが storage_dir の既存 .md を「外部削除」したとき、手動 Refresh / 再起動なしに
//     NoteFeed UI から当該 Block が除去されること。A だけが消えて B が残る (remove_note(&A.id))。
//     これが watcher 検知 → NoteFileDeletedExternally → subscriber (remove_one + notes-changed) →
//     frontend 再 hydrate の縦断 pipeline の間接観測になる (domain event 自体は frontend に露出しない)。
//
// 注意: list_notes は毎回 disk から再 hydrate するため、テスト本体で手動 invoke して
//      「A が見えない」ことを確認しても watcher の証明にはならない。よって step 1 / 3 は
//      list_notes を呼ばず DOM の自動更新のみで判定する (spec.md#impl-notes)。

import { writeFileSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;

const NOTE_A = '20260620120000';
const NOTE_B = '20260621090000';
const NOTE_NON_NOTE_NAME = 'notanote';

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

function deleteNoteMd(id: string): void {
  unlinkSync(join(NOTES_DIR, `${id}.md`));
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

async function waitForBlockIds(expected: string[], timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    async () => {
      const ids = await blockIds();
      if (ids.length !== expected.length || !ids.every((id, i) => id === expected[i])) return false;
      if (expected.length === 0) {
        return await $('[data-testid="screen-1-feed-empty"]').isExisting();
      }
      return true;
    },
    {
      timeout: timeoutMs,
      interval: 200,
      timeoutMsg: `feed blocks did not become [${expected.join(', ')}]`,
    },
  );
}

async function invokeListNotes(): Promise<NoteFeed> {
  const result = await browser.tauri.execute(async (tauri) => {
    return await tauri.core.invoke('list_notes');
  });
  return result as NoteFeed;
}

describe('scenario:s18-external-file-deleted', () => {
  before(async () => {
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({ timeout: 15000, timeoutMsg: 'seeded note A did not appear in feed' });
    const blockB = await $(`[data-block-id="${NOTE_B}"]`);
    await blockB.waitForExist({ timeout: 15000, timeoutMsg: 'seeded note B did not appear in feed' });

    // Given: storage_dir には Note A / Note B → フィード表示 2 件 (既定 sort createdAt desc)
    expect(await blockIds()).toEqual([NOTE_B, NOTE_A]);
  });

  it('step 1 — validation#s18-external-file-deleted: 外部 .md 削除で当該 Block が自動除去される', async () => {
    // When: 外部プログラムの代役として既存 .md を削除する
    //       (list_notes の手動 invoke / 再起動は一切行わない)
    deleteNoteMd(NOTE_A);

    // Then: watcher 検知 → NoteFileDeletedExternally → remove_note(&A.id) → notes-changed →
    //       再 hydrate により Block A だけが DOM から消え、Block B は残る
    await waitForBlockIds([NOTE_B]);
    expect(await $(`[data-block-id="${NOTE_A}"]`).isExisting()).toBe(false);

    // UI 通知は不要 (トーストが出ない)
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 2 — validation#s18-external-file-deleted: 除去後の read DTO が正しい', async () => {
    // When: 反映後の read DTO を取得する
    const feed = await invokeListNotes();

    // Then: Note A は含まれず、Note B のみ残る
    expect(feed.notes.find((n) => n.id === NOTE_A)).toBeUndefined();
    const noteB = feed.notes.find((n) => n.id === NOTE_B);
    expect(noteB).toBeDefined();
    expect(noteB?.body).toBe('world');
    expect(noteB?.tags).toEqual([]);
    expect(noteB?.created_at).toBe('2026-06-21T09:00:00Z');
    expect(noteB?.updated_at).toBe('2026-06-21T09:00:00Z');
  });

  it('step 3 — validation#s18-external-file-deleted: 連続する外部削除も反映されフィードが空になる', async () => {
    // When: debounce 窓 (500ms) を超えて待ってから、残りの Note B も外部削除する (手動操作なし)
    await browser.pause(700);
    deleteNoteMd(NOTE_B);

    // Then: DOM から Block B も消え、フィードは空 (empty 表示) になる
    await waitForBlockIds([]);
    expect(await $(`[data-block-id="${NOTE_B}"]`).isExisting()).toBe(false);
  });

  it('step 4 — validation#s18-external-file-deleted: 非 Note ファイル名の .md 削除は無視される', async () => {
    // When: `^\d{14}$` に一致しない .md を外部作成 → 削除する
    writeFileSync(join(NOTES_DIR, `${NOTE_NON_NOTE_NAME}.md`), 'not a note\n', 'utf-8');
    deleteNoteMd(NOTE_NON_NOTE_NAME);

    // Then: resolve_note_id が None のため event 非発行で skip され、フィードは空のまま
    //       (弱い観測: 非 Note 名は list_all 側でも feed に現れない。notes.md#known-issues 参照)
    await browser.pause(1500);
    expect(await blockIds()).toEqual([]);
    expect(await $('[data-testid="screen-1-feed-empty"]').isExisting()).toBe(true);
  });
});
