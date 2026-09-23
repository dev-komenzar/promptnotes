// @ori-generated scenario:s7-undo-after-toast
//
// 検証対象: validation.md#s7-undo-after-toast
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に
//        Note A (20260620120000, body=alpha) / Note B (20260620130000, body=bravo) を seed する

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

function notePath(storageDir: string, id: string): string {
  return join(storageDir, `${id}.md`);
}

function trashPath(storageDir: string, id: string): string {
  return join(storageDir, 'trash', `${id}.md`);
}

async function count(selector: string): Promise<number> {
  const els = await $$(selector);
  return (els as unknown as { length: number }).length;
}

async function dispatchClick(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    const el = document.querySelector<HTMLElement>(sel);
    el?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  }, selector);
}

type RestoreAttempt =
  | { ok: true; value: unknown }
  | { ok: false; error: { kind?: string; id?: string } | unknown };

type PendingRestore = { ok: 'pending' };

/**
 * `restore_deleted_note` Tauri command をブラウザ文脈から直接 invoke し、
 * reject payload を観測する。Toast 消失後は Undo ボタンが DOM に無いため、
 * S7 の「復元 API 呼び出し試行」はこの経路で行う。
 *
 * `@wdio/tauri-service` (driverProvider=external) の patchedExecute は
 * `browser.executeAsync` を扱えないため、同期 `browser.execute` で kick-off し
 * window 上の結果を poll する。poll は WebKitWebDriver の serialization 制約を
 * 避けるため**常に文字列**を返す。
 */
async function invokeRestore(id: string): Promise<RestoreAttempt> {
  await browser.execute((noteId: string) => {
    const w = window as unknown as {
      __TAURI_INTERNALS__?: {
        invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
      __oriS7Restore?: RestoreAttempt | PendingRestore | string;
    };
    w.__oriS7Restore = 'pending';
    const invoke = w.__TAURI_INTERNALS__?.invoke;
    if (!invoke) {
      w.__oriS7Restore = JSON.stringify({ ok: false, error: { kind: 'no_tauri_internals' } });
      return;
    }
    invoke('restore_deleted_note', { noteId })
      .then((value) => {
        w.__oriS7Restore = JSON.stringify({ ok: true, value });
      })
      .catch((error: unknown) => {
        w.__oriS7Restore = JSON.stringify({ ok: false, error });
      });
  }, id);

  for (let i = 0; i < 50; i++) {
    const raw = await browser.execute((key: string) => {
      const store = window as unknown as Record<string, unknown>;
      const value = store[key];
      return typeof value === 'string' ? value : 'pending';
    }, '__oriS7Restore');
    if (raw !== 'pending') {
      return JSON.parse(raw) as RestoreAttempt;
    }
    await browser.pause(100);
  }
  throw new Error('restore_deleted_note result was not observed (S7 t2)');
}

describe('scenario:s7-undo-after-toast', () => {
  const STORAGE_DIR = process.env.TAURI_TEST_STORAGE_DIR!;
  const A = '20260620120000';
  const B = '20260620130000';
  const PATH_A = notePath(STORAGE_DIR, A);
  const PATH_B = notePath(STORAGE_DIR, B);

  const deleteBtn = (id: string) =>
    `[data-block-id="${id}"] [data-testid="screen-1-block-delete"]`;
  const undoBtn = (id: string) =>
    `[data-testid="screen-1-toast-undo"][data-toast-id="${id}"]`;
  const toast = (id: string) =>
    `[data-testid="screen-1-toast"][data-toast-id="${id}"]`;

  before(async () => {
    await browser.waitUntil(async () => (await count('[data-testid="screen-1-block"]')) >= 2, {
      timeout: 10000,
      timeoutMsg: 'seeded notes A/B did not appear in feed',
    });
    await browser.pause(1500);
  });

  it('rejects undo for an expired toast (NoUndoAvailable) while other toasts stay valid', async () => {
    // ---- Given: A と B が表示され、どちらも未削除 ----
    expect(await count('[data-testid="screen-1-block"]')).toBe(2);
    expect(existsSync(PATH_A)).toBe(true);
    expect(existsSync(PATH_B)).toBe(true);
    expect(await count('[data-testid="screen-1-toast"]')).toBe(0);

    // ---- t0: A を削除 ----
    await dispatchClick(deleteBtn(A));
    await browser.pause(1000);

    expect(existsSync(PATH_A)).toBe(false);
    expect(existsSync(trashPath(STORAGE_DIR, A))).toBe(true);
    expect(await count('[data-testid="screen-1-block"]')).toBe(1);
    expect(await count(toast(A))).toBe(1);

    // ---- t0 + 3s: A の Toast 表示中に B を削除 ----
    await browser.pause(2000);
    await dispatchClick(deleteBtn(B));
    await browser.pause(500);

    expect(existsSync(PATH_B)).toBe(false);
    expect(await count('[data-testid="screen-1-block"]')).toBe(0);
    expect(await count('[data-testid="screen-1-toast"]')).toBe(2);
    expect(await count(toast(A))).toBe(1);
    expect(await count(toast(B))).toBe(1);

    // ---- t1: A の Toast が有効期間切れ (5s) で消失。B の Toast は残る ----
    await browser.waitUntil(async () => (await count(toast(A))) === 0, {
      timeout: 8000,
      interval: 250,
      timeoutMsg: "A's toast did not disappear after its TTL (5s)",
    });
    expect(await count(toast(B))).toBe(1);
    expect(existsSync(PATH_A)).toBe(false);

    // Rust 側 Undo スタックの per-element TTL (5s) 経過を確実にする
    await browser.pause(1000);

    // ---- t2: A への復元 API 試行 → NoUndoAvailable ----
    const attempt = await invokeRestore(A);
    expect(attempt.ok).toBe(false);
    expect((attempt as { error: { kind?: string } }).error.kind).toBe('no_undo_available');
    // A はゴミ箱に残ったまま（原パスへ復帰しない）
    expect(existsSync(PATH_A)).toBe(false);
    expect(existsSync(trashPath(STORAGE_DIR, A))).toBe(true);
    expect(await count(toast(A))).toBe(0);

    // ---- per-toast 独立性: B の Undo は引き続き有効 ----
    await dispatchClick(undoBtn(B));
    await browser.pause(1000);

    expect(existsSync(PATH_B)).toBe(true);
    expect(readFileSync(PATH_B, 'utf-8')).toContain('bravo');
    expect(await count(toast(B))).toBe(0);
    expect(await count('[data-testid="screen-1-toast"]')).toBe(0);
    expect(await count('[data-testid="screen-1-block"]')).toBe(1);

    // A は依然として復元されていない
    expect(existsSync(PATH_A)).toBe(false);
  });
});
