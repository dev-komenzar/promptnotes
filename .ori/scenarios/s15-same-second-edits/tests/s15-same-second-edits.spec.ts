// @ori-generated scenario:s15-same-second-edits
//
// 検証対象: validation.md#s15-same-second-edits
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME と storage_dir を隔離し、
//        Note A (20260620120000, body="hello") と Note B (20260620110000, body="beta") を seed する。
//        AutoSave の now は debug seam (TAURI_TEST_FIXED_NOW_FILE) で固定秒に pin できる。
//
// 核: Note::edit_body の now は秒精度 Timestamp に truncate されるため、同一秒内の連続編集では
//     updatedAt は同じ値に留まる (I-N4)。ただし body は変化しているので persist と
//     NoteBodyEdited 発行は実行される (S9 の body 同値 no-op とは逆)。step 3 の陽性対照
//     (秒が変われば updatedAt が進む) により step 1-2 の据え置きが空振りでないことを示す。
//
// now の制御 (spec.md#impl-notes):
//   production の release build は seam を compile out するため、本 seam は test build (debug)
//   でのみ有効。テストはステップ間に固定時刻ファイルへ RFC3339 を書いて now を切り替える。

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_STORAGE_DIR!;
const FIXED_NOW_FILE = process.env.TAURI_TEST_FIXED_NOW_FILE!;

const NOTE_A = '20260620120000';
const NOTE_B = '20260620110000';
const T0 = '20260620120000';
const SAME_SECOND = '2026-06-20T12:00:00Z';

type AutoSaveOutcome =
  | { outcome: 'saved'; id: string; updated_at: string }
  | { outcome: 'no_op' };

type NoteSummary = {
  id: string;
  body: string;
  tags: string[];
  created_at: string;
  updated_at: string;
};

type NoteFeed = { notes: NoteSummary[] };

function pinNow(rfc3339: string): void {
  writeFileSync(FIXED_NOW_FILE, rfc3339);
}

function readMd(id: string): string {
  return readFileSync(join(NOTES_DIR, `${id}.md`), 'utf-8');
}

function extractUpdatedAt(md: string): string | null {
  const m = md.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

function bodyOf(md: string): string {
  const parts = md.split('---\n');
  return (parts[2] ?? '').trimEnd();
}

async function invokeAutoSave(noteId: string, newBody: string): Promise<AutoSaveOutcome> {
  const result = await browser.tauri.execute(
    async (tauri, id: string, body: string) => {
      return await tauri.core.invoke('auto_save_note', { noteId: id, newBody: body });
    },
    noteId,
    newBody,
  );
  return result as AutoSaveOutcome;
}

async function invokeListNotes(): Promise<NoteFeed> {
  const result = await browser.tauri.execute(async (tauri) => {
    return await tauri.core.invoke('list_notes');
  });
  return result as NoteFeed;
}

describe('scenario:s15-same-second-edits', () => {
  before(async () => {
    const blockA = await $(`[data-block-id="${NOTE_A}"]`);
    await blockA.waitForExist({ timeout: 15000, timeoutMsg: 'seeded note A did not appear in feed' });
    await browser.pause(500);

    expect(bodyOf(readMd(NOTE_A))).toBe('hello');
    expect(extractUpdatedAt(readMd(NOTE_A))).toBe(T0);
  });

  it('step 1 — validation#s15-same-second-edits: 1 回目 (12:00:00.100) は永続化するが updatedAt は据え置き', async () => {
    // Given: seed と同一秒の now に pin
    pinNow('2026-06-20T12:00:00.100Z');

    // When: body を変更して auto_save_note を invoke (Note::edit_body)
    const res = await invokeAutoSave(NOTE_A, 'hello v1');

    // Then: persist + publish は実行され (outcome=saved)、updated_at は seed の秒と同値 (I-N4)
    expect(res.outcome).toBe('saved');
    if (res.outcome === 'saved') {
      expect(res.id).toBe(NOTE_A);
      expect(res.updated_at).toBe(SAME_SECOND);
    }
    const after = readMd(NOTE_A);
    expect(bodyOf(after)).toBe('hello v1');
    expect(extractUpdatedAt(after)).toBe(T0);
  });

  it('step 2 — validation#s15-same-second-edits: 2 回目 (12:00:00.800) も同一秒内なので updatedAt は不変', async () => {
    // Given: 依然として同一秒 (サブ秒のみ進行) の now に pin
    pinNow('2026-06-20T12:00:00.800Z');

    // When: body をさらに変更して auto_save_note を invoke
    const res = await invokeAutoSave(NOTE_A, 'hello v2');

    // Then: persist + publish は実行されるが updatedAt は 12:00:00 のまま
    expect(res.outcome).toBe('saved');
    if (res.outcome === 'saved') {
      expect(res.updated_at).toBe(SAME_SECOND);
    }
    const after = readMd(NOTE_A);
    expect(bodyOf(after)).toBe('hello v2');
    expect(extractUpdatedAt(after)).toBe(T0);
  });

  it('step 3 — validation#s15-same-second-edits: NoteFeed は再 sort しても冪等 (2 回実行で同一順序)', async () => {
    // When: 2 回の同一秒編集後に list_notes を 2 回 invoke (sort 再計算)
    const first = await invokeListNotes();
    const second = await invokeListNotes();

    // Then: 同一順序 (CreatedAt desc) / 同一 updated_at を返し、A の updatedAt は据え置きのまま
    expect(first.notes.map((n) => n.id)).toEqual([NOTE_A, NOTE_B]);
    expect(second.notes.map((n) => n.id)).toEqual([NOTE_A, NOTE_B]);
    expect(second.notes).toEqual(first.notes);

    const a = first.notes.find((n) => n.id === NOTE_A);
    expect(a?.updated_at).toBe(SAME_SECOND);
    expect(a?.body).toBe('hello v2');
  });

  it('step 4 — validation#s15-same-second-edits: 陽性対照 — 秒が変われば updatedAt は進む', async () => {
    // Given: 次の秒 (12:00:01) に進める
    pinNow('2026-06-20T12:00:01Z');

    // When: 同じ auto_save_note を invoke
    const res = await invokeAutoSave(NOTE_A, 'hello v3');

    // Then: updated_at が 12:00:01 に進み、永続化ファイルの updatedAt も変わる。
    //   これが step 1-2 の据え置きの非空虚性 (秒精度 truncate による同値) を保証する。
    expect(res.outcome).toBe('saved');
    if (res.outcome === 'saved') {
      expect(res.updated_at).toBe('2026-06-20T12:00:01Z');
    }
    const after = readMd(NOTE_A);
    expect(bodyOf(after)).toBe('hello v3');
    expect(extractUpdatedAt(after)).toBe('20260620120001');
    expect(extractUpdatedAt(after)).not.toBe(T0);
  });
});
