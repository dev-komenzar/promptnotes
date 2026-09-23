// @ori-generated scenario:s21-batch-sync-debounce
//
// 検証対象: validation.md#s21-batch-sync-debounce
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A / B / C (20260620120000 / 20260620130000 / 20260620140000) を seed する。
//        app は起動時に PageMain から start_file_watcher を invoke し storage_dir を監視する。
//
// 核: Syncthing の一括同期を模し、debounce 窓 (500ms) 以内に A.md / B.md / C.md の 3 ファイルを
//     連続上書きしたとき、手動 Refresh / 再起動なしで 3 つの既存 Block の body がすべて自動更新される
//     こと (取りこぼしなし)。これが watcher 検知 (path 単位 debounce) → 再 parse →
//     NoteFileModifiedExternally ×3 → subscriber (upsert_one + notes-changed) → frontend 再 hydrate
//     の縦断 pipeline の間接観測になる (domain event 自体は frontend に露出しない)。
//
// 注意: list_notes は毎回 disk から再 hydrate するため、テスト本体で手動 invoke して
//      「変更後 body が見える」ことを確認しても watcher の証明にはならない。よって step 1 / 3 / 5 は
//      list_notes を呼ばず DOM の自動更新のみで判定する (spec.md#impl-notes)。

import { writeFileSync, renameSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;

const NOTE_A = '20260620120000';
const NOTE_B = '20260620130000';
const NOTE_C = '20260620140000';
const NOTE_D = '20260630120000';

const BODY_A_INITIAL = 'hello A';
const BODY_B_INITIAL = 'hello B';
const BODY_C_INITIAL = 'hello C';

const BODY_A_SYNC = 'hello A sync';
const BODY_B_SYNC = 'hello B sync';
const BODY_C_SYNC = 'hello C sync';

const BODY_D = 'hello D via rename';

// 既定 sort は created_at desc → C (14:00) > B (13:00) > A (12:00)
const DEFAULT_ORDER = [NOTE_C, NOTE_B, NOTE_A];

type NoteSummary = {
  id: string;
  body: string;
  tags: string[];
  created_at: string;
  updated_at: string;
};

type NoteFeed = { notes: NoteSummary[] };

function noteMd(id: string, body: string, tags: string[] = []): string {
  return [
    '---',
    `createdAt: ${id}`,
    `updatedAt: ${id}`,
    `tags: [${tags.join(', ')}]`,
    '---',
    body,
  ].join('\n');
}

function writeNoteMd(id: string, body: string): void {
  writeFileSync(join(NOTES_DIR, `${id}.md`), noteMd(id, body), 'utf-8');
}

function writeTmpNoteMd(id: string, body: string): void {
  writeFileSync(join(NOTES_DIR, `${id}.md.tmp`), noteMd(id, body), 'utf-8');
}

function renameTmpNoteMd(id: string): void {
  renameSync(join(NOTES_DIR, `${id}.md.tmp`), join(NOTES_DIR, `${id}.md`));
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
    }
  );
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

async function waitForAllBlockBodies(
  entries: ReadonlyArray<readonly [string, string]>,
  timeoutMs = 10_000
): Promise<void> {
  await browser.waitUntil(
    async () => {
      for (const [id, body] of entries) {
        if ((await blockBodyText(id)) !== body) return false;
      }
      return true;
    },
    {
      timeout: timeoutMs,
      interval: 200,
      timeoutMsg: `not all blocks reached expected bodies: ${entries
        .map(([id, body]) => `${id}="${body}"`)
        .join(', ')}`,
    }
  );
}

async function invokeListNotes(): Promise<NoteFeed> {
  const result = await browser.tauri.execute(async (tauri) => {
    return await tauri.core.invoke('list_notes');
  });
  return result as NoteFeed;
}

describe('scenario:s21-batch-sync-debounce', () => {
  before(async () => {
    const blockC = await $(`[data-block-id="${NOTE_C}"]`);
    await blockC.waitForExist({
      timeout: 15_000,
      timeoutMsg: 'seeded note C did not appear in feed',
    });
    await browser.pause(500);

    // Given: Note A / B / C が表示され、既定 sort (created_at desc) で [C, B, A]、全て IDLE
    expect(await blockIds()).toEqual(DEFAULT_ORDER);
    for (const id of DEFAULT_ORDER) {
      expect(await blockState(id)).toBe('IDLE');
    }
    await waitForBlockBody(NOTE_A, BODY_A_INITIAL);
    await waitForBlockBody(NOTE_B, BODY_B_INITIAL);
    await waitForBlockBody(NOTE_C, BODY_C_INITIAL);
  });

  it('step 1 — validation#s21-batch-sync-debounce: debounce 窓内の一括変更で 3 Block が全件自動更新される', async () => {
    // When: 外部プログラム (Syncthing) の代役として A.md / B.md / C.md を debounce 窓 (500ms)
    //       以内に連続上書きする (writeFileSync を間隔を置かず 3 回)。
    //       list_notes の手動 invoke / 再起動は一切行わない。
    writeNoteMd(NOTE_A, BODY_A_SYNC);
    writeNoteMd(NOTE_B, BODY_B_SYNC);
    writeNoteMd(NOTE_C, BODY_C_SYNC);

    // Then: watcher 検知 (path 単位 debounce) → 再 parse → notes-changed → 再 hydrate により
    //       3 つの既存 Block の DOM body がすべて自動更新される (取りこぼしなし)
    await waitForAllBlockBodies([
      [NOTE_A, BODY_A_SYNC],
      [NOTE_B, BODY_B_SYNC],
      [NOTE_C, BODY_C_SYNC],
    ]);

    // Then: upsert は差し替え (append でない) → Block 数 3・表示順も不変 (I-F8)
    expect(await blockIds()).toEqual(DEFAULT_ORDER);

    // Then: UI 通知は不要 (トーストが出ない)
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 2 — validation#s21-batch-sync-debounce: 反映された各 Note の内容が正しい', async () => {
    // When: 反映後の read DTO を取得する
    const feed = await invokeListNotes();
    const byId = new Map(feed.notes.map((note) => [note.id, note]));

    // Then: A / B / C それぞれの body / tags / 秒精度タイムスタンプが一致する
    //       (When は body のみ変更 → createdAt / updatedAt は Given の値のまま)
    const expected: Array<[string, string, string]> = [
      [NOTE_A, BODY_A_SYNC, '2026-06-20T12:00:00Z'],
      [NOTE_B, BODY_B_SYNC, '2026-06-20T13:00:00Z'],
      [NOTE_C, BODY_C_SYNC, '2026-06-20T14:00:00Z'],
    ];
    for (const [id, body, seedTimestamp] of expected) {
      const note = byId.get(id);
      expect(note).toBeDefined();
      expect(note?.body).toBe(body);
      expect(note?.tags).toEqual([]);
      expect(note?.created_at).toBe(seedTimestamp);
      expect(note?.updated_at).toBe(seedTimestamp);
    }
  });

  it('step 3 — validation#s21-batch-sync-debounce: 同一内容の再バッチでも Block は増えず表示が安定する', async () => {
    // When: debounce 窓 (500ms) を超えて待ってから、同じ 3 ファイルを同一 body で再度上書きする
    //       (外部同期の再送を模す)。
    await browser.pause(700);
    writeNoteMd(NOTE_A, BODY_A_SYNC);
    writeNoteMd(NOTE_B, BODY_B_SYNC);
    writeNoteMd(NOTE_C, BODY_C_SYNC);
    await browser.pause(1200);

    // Then: upsert_note は冪等 (I-F8) → Block は重複せず 3 のまま、body・表示順も不変
    expect(await blockIds()).toEqual(DEFAULT_ORDER);
    await waitForAllBlockBodies([
      [NOTE_A, BODY_A_SYNC],
      [NOTE_B, BODY_B_SYNC],
      [NOTE_C, BODY_C_SYNC],
    ]);
  });

  it('step 4 — validation#s21-batch-sync-debounce: IDLE のため競合ダイアログも通知も出ない', async () => {
    // Then: 全 Block は IDLE 状態のまま (編集中ではない → S19 の競合ケースではない)
    for (const id of DEFAULT_ORDER) {
      expect(await blockState(id)).toBe('IDLE');
    }

    // Then: 競合ダイアログ・トーストいずれも表示されない
    expect(await $('[data-testid="widget-external-change-conflict"]').isExisting()).toBe(false);
    expect((await $$('[data-testid="screen-1-toast"]')).length).toBe(0);
  });

  it('step 5 — validation#s21-batch-sync-debounce: .tmp は無視され rename 後の .md のみ処理される', async () => {
    // When: Syncthing 方式の一時ファイル D.md.tmp を書き込む (debounce 窓を超えて待つ)
    writeTmpNoteMd(NOTE_D, BODY_D);
    await browser.pause(900);

    // Then: .tmp は watcher に無視され、Block D は出現しない (feed は 3 のまま)
    expect(await blockIds()).toEqual(DEFAULT_ORDER);
    expect(await $(`[data-block-id="${NOTE_D}"]`).isExisting()).toBe(false);

    // When: rename で D.md.tmp → D.md に確定させる
    renameTmpNoteMd(NOTE_D);

    // Then: rename 完了後の .md のみが処理され、Block D が新規 Note として反映される
    //       (既定 sort created_at desc のため D = 20260630 が先頭)
    await waitForBlockIds([NOTE_D, NOTE_C, NOTE_B, NOTE_A]);
    await waitForBlockBody(NOTE_D, BODY_D);
  });
});