// @ori-generated scenario:s16-external-file-created
//
// 検証対象: validation.md#s16-external-file-created
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A (20260620120000, body="hello") を seed する。app は起動時に
//        PageMain から start_file_watcher を invoke し、storage_dir を監視する。
//
// 核: テストプロセスが storage_dir に .md を「外部作成」したとき、手動 Refresh / 再起動なしに
//     NoteFeed UI が自動で 1 件 → 2 件に更新されること。これが watcher 検知 →
//     NoteFileCreatedExternally → subscriber (upsert_one + notes-changed) → frontend 再 hydrate
//     の縦断 pipeline の間接観測になる (domain event 自体は frontend に露出しない)。
//
// 注意: list_notes は毎回 disk から再 hydrate するため、テスト本体で手動 invoke して
//      「新規 Note が見える」ことを確認しても watcher の証明にはならない。よって step 1 / 3 は
//      list_notes を呼ばず DOM の自動更新のみで判定する (spec.md#impl-notes)。

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;

const NOTE_A = '20260620120000';
const NOTE_NEW = '20260630120000';
const NOTE_NEW2 = '20260701090000';
const NOTE_MALFORMED = '20260630130000';

const NEW_BODY = '外部から作成';

type NoteSummary = {
  id: string;
  body: string;
  tags: string[];
  created_at: string;
  updated_at: string;
};

type NoteFeed = { notes: NoteSummary[] };

function writeNoteMd(id: string, body: string, tags: string[]): void {
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

function writeMalformedMd(id: string): void {
  const content = [
    '---',
    `createdAt: ${id}`,
    `updatedAt: ${id}`,
    'tags: [broken',
    'body without closing delimiter',
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

async function waitForBlockIds(expected: string[], timeoutMs = 10_000): Promise<void> {
  await browser.waitUntil(
    async () => {
      const ids = await blockIds();
      return ids.length === expected.length && ids.every((id, i) => id === expected[i]);
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

describe('scenario:s16-external-file-created', () => {
  before(async () => {
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({ timeout: 15000, timeoutMsg: 'seeded note A did not appear in feed' });
    await browser.pause(500);

    // Given: storage_dir には Note A のみ → フィード表示 1 件
    expect(await blockIds()).toEqual([NOTE_A]);
  });

  it('step 1 — validation#s16-external-file-created: 外部 .md 新規作成で NoteFeed が 1 件 → 2 件に自動更新', async () => {
    // When: 外部プログラムの代役として storage_dir に新規 .md を作成する
    //       (list_notes の手動 invoke / 再起動は一切行わない)
    writeNoteMd(NOTE_NEW, NEW_BODY, ['rust']);

    // Then: watcher 検知 → NoteFileCreatedExternally → notes-changed → 再 hydrate により
    //       DOM に新規 block が出現する。既定 sort は createdAt desc なので新規が先頭。
    await waitForBlockIds([NOTE_NEW, NOTE_A]);

    // UI 通知は不要 (トーストが出ない)
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 2 — validation#s16-external-file-created: 反映された Note の内容が正しい', async () => {
    // When: 反映後の read DTO を取得する
    const feed = await invokeListNotes();

    // Then: parse された body / tags / 秒精度タイムスタンプが一致する
    const created = feed.notes.find((n) => n.id === NOTE_NEW);
    expect(created).toBeDefined();
    expect(created?.body).toBe(NEW_BODY);
    expect(created?.tags).toEqual(['rust']);
    expect(created?.created_at).toBe('2026-06-30T12:00:00Z');
    expect(created?.updated_at).toBe('2026-06-30T12:00:00Z');
  });

  it('step 3 — validation#s16-external-file-created: 連続する外部作成もそれぞれ自動反映される', async () => {
    // When: 2 つ目の外部ファイルを作成する (手動操作なし)
    writeNoteMd(NOTE_NEW2, '二つ目', []);

    // Then: UI が 3 件になり、sort (createdAt desc) が維持された順序で表示される
    await waitForBlockIds([NOTE_NEW2, NOTE_NEW, NOTE_A]);
  });

  it('step 4 — validation#s16-external-file-created: malformed .md は parse 失敗で skip される', async () => {
    // When: 壊れた frontmatter の .md を外部作成する
    writeMalformedMd(NOTE_MALFORMED);

    // Then: debounce 窓 (500ms) を十分に超えて待ってもフィード件数は不変 (event 未発行 → skip)
    await browser.pause(1500);
    expect(await blockIds()).toEqual([NOTE_NEW2, NOTE_NEW, NOTE_A]);
    expect(await $(`[data-block-id="${NOTE_MALFORMED}"]`).isExisting()).toBe(false);
  });
});
