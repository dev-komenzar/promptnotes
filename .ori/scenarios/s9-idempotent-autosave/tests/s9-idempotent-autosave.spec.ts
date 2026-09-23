// @ori-generated scenario:s9-idempotent-autosave
//
// 検証対象: validation.md#s9-idempotent-autosave
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に
//        Note A (20260620120000, body="hello", updatedAt=t0) を seed する
//
// 核: EDITING 状態で body が変化していなければ AutoSave 経路は何もしない
//     (Note::edit_body 非呼出 / write なし / NoteBodyEdited 非発行 / updatedAt=t0)。
//     対照として body 変化時は Saved になり updatedAt が更新されることを確認する。

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTE_ID = '20260620120000';
const T0 = '20260620120000';
const MD_PATH = join(process.env.TAURI_TEST_STORAGE_DIR!, `${NOTE_ID}.md`);

function readMd(): string {
  return readFileSync(MD_PATH, 'utf-8');
}

function extractUpdatedAt(md: string): string | null {
  const m = md.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

function bodyOf(md: string): string {
  const parts = md.split('---\n');
  return (parts[2] ?? '').trimEnd();
}

type AutoSaveOutcome =
  | { outcome: 'saved'; id: string; updated_at: string }
  | { outcome: 'no_op' };

type InvokeResult = { ok: true; value: AutoSaveOutcome } | { ok: false; error: unknown };

/**
 * `auto_save_note` Tauri command をブラウザ文脈から直接 invoke し、AutoSave 経路の
 * application service ガード (C-AS3 / S9) を Tauri 境界で観測する。
 *
 * `@wdio/tauri-service` (driverProvider=external) の patchedExecute は
 * `browser.executeAsync` を扱えないため、同期 `browser.execute` で kick-off し
 * window 上の結果を poll する (S7 / S8 と同方式)。poll は WebKitWebDriver の
 * serialization 制約を避けるため**常に文字列**を返す。
 */
async function invokeAutoSave(newBody: string): Promise<AutoSaveOutcome> {
  await browser.execute(
    (noteId: string, body: string) => {
      const w = window as unknown as {
        __TAURI_INTERNALS__?: {
          invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
        __oriS9AutoSave?: string;
      };
      w.__oriS9AutoSave = 'pending';
      const invoke = w.__TAURI_INTERNALS__?.invoke;
      if (!invoke) {
        w.__oriS9AutoSave = JSON.stringify({ ok: false, error: { kind: 'no_tauri_internals' } });
        return;
      }
      invoke('auto_save_note', { noteId, newBody: body })
        .then((value) => {
          w.__oriS9AutoSave = JSON.stringify({ ok: true, value });
        })
        .catch((error: unknown) => {
          w.__oriS9AutoSave = JSON.stringify({ ok: false, error });
        });
    },
    NOTE_ID,
    newBody,
  );

  for (let i = 0; i < 50; i++) {
    const observed = await browser.execute((key: string) => {
      const store = window as unknown as Record<string, unknown>;
      return typeof store[key] === 'string' ? (store[key] as string) : 'pending';
    }, '__oriS9AutoSave');
    if (observed !== 'pending') {
      const parsed = JSON.parse(observed) as InvokeResult;
      if (!parsed.ok) {
        throw new Error(`auto_save_note rejected: ${JSON.stringify(parsed.error)}`);
      }
      return parsed.value;
    }
    await browser.pause(100);
  }
  throw new Error('auto_save_note result was not observed (S9)');
}

describe('scenario:s9-idempotent-autosave', () => {
  before(async () => {
    const block = await $(`[data-block-id="${NOTE_ID}"]`);
    await block.waitForExist({ timeout: 10000, timeoutMsg: 'seeded note A did not appear in feed' });
    await browser.pause(800);

    expect(bodyOf(readMd())).toBe('hello');
    expect(extractUpdatedAt(readMd())).toBe(T0);
  });

  it('step 1 — EDITING 遷移のみ（body 不変）では AutoSave が発火しない', async () => {
    const before = readMd();

    const block = await $(`[data-block-id="${NOTE_ID}"]`);
    await block.click();
    await browser.waitUntil(
      async () => (await block.getAttribute('data-block-state')) === 'EDITING',
      { timeout: 5000, timeoutMsg: 'block did not enter EDITING' },
    );

    // debounce 区間 (500ms) を超えて待機しても永続化されない
    await browser.pause(900);

    const after = readMd();
    expect(bodyOf(after)).toBe('hello');
    expect(extractUpdatedAt(after)).toBe(T0);
    expect(after).toBe(before);
  });

  it('step 2 — 同一 body の auto_save_note は no_op（S9 の核）', async () => {
    const before = readMd();

    const outcome = await invokeAutoSave('hello');
    expect(outcome.outcome).toBe('no_op');

    const after = readMd();
    expect(after).toBe(before);
    expect(bodyOf(after)).toBe('hello');
    expect(extractUpdatedAt(after)).toBe(T0);
  });

  it('step 3 — body 変化時は Saved になり updatedAt が更新される（対照）', async () => {
    const outcome = await invokeAutoSave('hello world');
    expect(outcome.outcome).toBe('saved');
    if (outcome.outcome === 'saved') {
      expect(outcome.id).toBe(NOTE_ID);
      expect(outcome.updated_at).not.toBe(T0);
    }

    const after = readMd();
    expect(bodyOf(after)).toBe('hello world');
    expect(extractUpdatedAt(after)).not.toBe(T0);
  });
});
