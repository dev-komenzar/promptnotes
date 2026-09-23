// @ori-generated scenario:s13-quit-flush
//
// 検証対象: validation.md#s13-quit-flush
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME/XDG_DATA_HOME と storage_dir を隔離し、
//        隔離 temp dir に Note A, B, C, D を seed する。
//
// 核: アプリ quit 時に debounce timer 待ちの pending_body を取りこぼさず Flush 経路で永続化する。
//     flush-note workflow (trigger=AppQuit) は debounce を待たず即時永続化し、body 不変時は no-op。
//
// quit トリガーの再現制約 (spec.md#impl-notes):
//   Tauri capability に `core:window:allow-close` が無いため、E2E から
//   `getCurrentWindow().close()` で CloseRequested を JS 発火できない。よって app_quit 経路は
//   `browser.tauri.execute(({ core }) => core.invoke('flush_note', { trigger: 'app_quit' }))` による
//   Tauri 境界 invoke で再現する（@wdio/tauri-service v1.4.0 の公式 API。tauri-plugin-wdio は debug で有効）。
//   frontend orchestration (pendingFlushRegistry / onCloseRequested) は既存 unit test が担保する。

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_S13_NOTES_DIR!;

const A = '20260620120000';
const B = '20260620130000';
const C = '20260620140000';
const D = '20260620150000';

type FlushOutcome = { outcome: 'flushed'; id: string; updated_at: string } | { outcome: 'no_op' };

function notePath(id: string): string {
  return join(NOTES_DIR, `${id}.md`);
}

function readNote(id: string): string {
  return readFileSync(notePath(id), 'utf-8');
}

function extractUpdatedAt(md: string): string | null {
  const m = md.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

function block(id: string) {
  return $(`[data-block-id="${id}"]`);
}

/**
 * app quit 相当の Flush を Tauri 境界で発火する。
 * `flush_note` command は trigger=app_quit を受理し、debounce を待たず即時永続化する。
 */
async function flushAppQuit(noteId: string, pendingBody: string): Promise<FlushOutcome> {
  const result = await browser.tauri.execute(
    async (tauri, id: string, body: string) => {
      return await tauri.core.invoke('flush_note', {
        noteId: id,
        pendingBody: body,
        trigger: 'app_quit',
      });
    },
    noteId,
    pendingBody,
  );
  return result as FlushOutcome;
}

describe('scenario:s13-quit-flush', () => {
  before(async () => {
    // Given: seed 済み Note A, B, C, D が feed に表示されるまで待つ（load-settings → list-feed 完了）
    await (await block(A)).waitForExist({ timeout: 15000 });
    await (await block(B)).waitForExist({ timeout: 15000 });
    await (await block(C)).waitForExist({ timeout: 15000 });
    await (await block(D)).waitForExist({ timeout: 15000 });
    await browser.pause(500);
  });

  it('step 1 — validation#s13-quit-flush: AppQuit Flush は A → B → C の順に逐次永続化し、各段階で当該 Note のみ更新する', async () => {
    // When/Then: A を app_quit で Flush。当該 Note のみ更新され、後続は未更新（逐次性）。
    const flushedA = await flushAppQuit(A, 'alpha quit-A');
    expect(flushedA.outcome).toBe('flushed');
    expect(readNote(A)).toContain('alpha quit-A');
    expect(readNote(B)).toContain('beta body');
    expect(readNote(C)).toContain('gamma body');

    // When/Then: B を app_quit で Flush。A の結果は保持され、C は未更新。
    const flushedB = await flushAppQuit(B, 'beta quit-B');
    expect(flushedB.outcome).toBe('flushed');
    expect(readNote(A)).toContain('alpha quit-A');
    expect(readNote(B)).toContain('beta quit-B');
    expect(readNote(C)).toContain('gamma body');

    // When/Then: C を app_quit で Flush。全 Note が更新済みになる（quit 完了時の全反映）。
    const flushedC = await flushAppQuit(C, 'gamma quit-C');
    expect(flushedC.outcome).toBe('flushed');
    expect(readNote(A)).toContain('alpha quit-A');
    expect(readNote(B)).toContain('beta quit-B');
    expect(readNote(C)).toContain('gamma quit-C');

    // Then: updated_at は seed 値から進む（I-N4）。
    expect(extractUpdatedAt(readNote(A))).not.toBe(A);
    expect(extractUpdatedAt(readNote(B))).not.toBe(B);
    expect(extractUpdatedAt(readNote(C))).not.toBe(C);
  });

  it('step 2 — validation#s13-quit-flush: body 不変の AppQuit Flush は no-op（冪等性ガード）', async () => {
    const before = readNote(A);
    const beforeUpdatedAt = extractUpdatedAt(before);

    // When: 既に flush 済みの body と同じ pending_body で app_quit Flush を発火
    const outcome = await flushAppQuit(A, 'alpha quit-A');

    // Then: no_op を返し、ファイルは不変（永続化も event 発行も無し）
    expect(outcome.outcome).toBe('no_op');
    expect(readNote(A)).toBe(before);
    expect(extractUpdatedAt(readNote(A))).toBe(beforeUpdatedAt);
  });

  it('step 3 — validation#s13-quit-flush: debounce 中の pending body を AppQuit Flush が quit 前に永続化する（欠損なし）', async () => {
    // Given: 実 UI で Note D を EDITING にし、debounce timer (500ms) 中の pending_body を作る。
    const el = await block(D);
    await el.waitForExist({ timeout: 10000 });
    await el.click();
    await browser.waitUntil(
      async () => (await el.getAttribute('data-block-state')) === 'EDITING',
      { timeout: 5000, timeoutMsg: 'block D did not enter EDITING' },
    );
    const editor = await el.$('.cm-editor .cm-content');
    await editor.waitForExist({ timeout: 3000 });
    await editor.click();

    const before = readNote(D);
    const beforeUpdatedAt = extractUpdatedAt(before);
    expect(before).toContain('delta body');

    // When: debounce (500ms) の成立を待たずに app_quit Flush を発火する。
    //   flush が debounce より先に永続化した場合のみ `flushed` が返る（debounce が先に fire して
    //   いれば body は既に同値となり `no_op` になる）。よって outcome=flushed は
    //   「debounce を待たず Flush が永続化した」ことの決定的な証拠になる。
    await browser.keys('!');
    const flushed = await flushAppQuit(D, 'delta body!');

    // Then: Flush が pending_body を永続化した（欠損なし、updated_at 更新）。
    expect(flushed.outcome).toBe('flushed');
    expect(readNote(D)).toContain('delta body!');
    expect(extractUpdatedAt(readNote(D))).not.toBe(beforeUpdatedAt);

    // Then: 後から debounce timer が fire しても同一 body のため重複永続化しない（冪等）。
    const afterFlushUpdatedAt = extractUpdatedAt(readNote(D));
    await browser.pause(900);
    expect(readNote(D)).toContain('delta body!');
    expect(extractUpdatedAt(readNote(D))).toBe(afterFlushUpdatedAt);
  });
});
