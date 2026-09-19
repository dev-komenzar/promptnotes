// @ori-generated scenario:s10-tag-invalid-char
//
// 検証対象: validation.md#s10-tag-invalid-char
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に
//        Note A (20260620120000, body="hello", tags=[], updatedAt=t0) を seed する
//
// 核: 禁止文字 (I-N6: ' ', '\t', '\n', ',', '[', ']') を含む Tag は
//     Tag::new 構築時に TagError::InvalidChar で reject され、Note::assign_tag には
//     到達しない (永続化なし / event NoteTagsChanged 非発行)。UI は invalid_tag を catch して
//     エラーメッセージを表示する。対照として正常タグは永続化されることを確認する。

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTE_ID = '20260620120000';
const T0 = '20260620120000';
const MD_PATH = join(process.env.TAURI_TEST_STORAGE_DIR!, `${NOTE_ID}.md`);

// I-N6 の禁止文字集合 (aggregates.md#note-aggregate / FORBIDDEN_TAG_CHARS)。
// UI の <input type="text"> では \t / \n を入力できないため Tauri 境界で全件検証する。
const FORBIDDEN_TAGS = ['foo bar', 'foo\tbar', 'foo\nbar', 'foo,bar', 'foo[bar', 'foo]bar'];

function readMd(): string {
  return readFileSync(MD_PATH, 'utf-8');
}

function extractTags(md: string): string[] {
  const m = md.match(/^tags:\s*\[(.*)\]$/m);
  if (!m) return [];
  const raw = m[1].replace(/"/g, '').trim();
  return raw ? raw.split(',').map((s) => s.trim()).filter(Boolean) : [];
}

function extractUpdatedAt(md: string): string | null {
  const m = md.match(/^updatedAt:\s*(.+)$/m);
  return m ? m[1].trim() : null;
}

function block() {
  return $(`[data-block-id="${NOTE_ID}"]`);
}

async function chipTexts(): Promise<string[]> {
  const texts = await browser.execute((id: string) => {
    const els = document.querySelectorAll(
      `[data-block-id="${id}"] [data-testid="screen-1-block-tag-chip"]`,
    );
    return Array.from(els).map((e) => (e.textContent ?? '').replace('#', '').trim());
  }, NOTE_ID);
  return texts as string[];
}

async function addTag(raw: string): Promise<void> {
  const el = await block();
  const input = await el.$('[data-testid="screen-1-block-tag-input"]');
  await input.waitForExist({ timeout: 3000 });
  await input.click();
  await input.setValue(raw);
  await browser.keys(['Enter']);
}

type AssignTagErrorDto = { kind: string; name?: string; reason?: string };
type InvokeResult = { ok: true; value: unknown } | { ok: false; error: AssignTagErrorDto };

/**
 * `assign_tag` Tauri command をブラウザ文脈から直接 invoke し、禁止文字 reject を
 * Tauri 境界で観測する。同期 `browser.execute` で kick-off し window 上の結果を poll する
 * (`@wdio/tauri-service` (driverProvider=external) の patchedExecute は browser.executeAsync を
 * 扱えないため。s7 / s8 / s9 と同方式)。
 */
async function invokeAssignTag(rawTag: string): Promise<InvokeResult> {
  await browser.execute(
    (noteId: string, raw: string) => {
      const w = window as unknown as {
        __TAURI_INTERNALS__?: {
          invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
        __oriS10AssignTag?: string;
      };
      w.__oriS10AssignTag = 'pending';
      const invoke = w.__TAURI_INTERNALS__?.invoke;
      if (!invoke) {
        w.__oriS10AssignTag = JSON.stringify({ ok: false, error: { kind: 'no_tauri_internals' } });
        return;
      }
      invoke('assign_tag', { noteId, rawTag: raw })
        .then((value) => {
          w.__oriS10AssignTag = JSON.stringify({ ok: true, value });
        })
        .catch((error: unknown) => {
          w.__oriS10AssignTag = JSON.stringify({ ok: false, error });
        });
    },
    NOTE_ID,
    rawTag,
  );

  for (let i = 0; i < 50; i++) {
    const observed = await browser.execute((key: string) => {
      const store = window as unknown as Record<string, unknown>;
      return typeof store[key] === 'string' ? (store[key] as string) : 'pending';
    }, '__oriS10AssignTag');
    if (observed !== 'pending') {
      return JSON.parse(observed) as InvokeResult;
    }
    await browser.pause(100);
  }
  throw new Error('assign_tag result was not observed (S10)');
}

describe('scenario:s10-tag-invalid-char', () => {
  before(async () => {
    const el = await block();
    await el.waitForExist({ timeout: 10000, timeoutMsg: 'seeded note A did not appear in feed' });
    await browser.pause(800);

    expect(extractTags(readMd())).toEqual([]);
    expect(extractUpdatedAt(readMd())).toBe(T0);

    await el.click();
    await browser.waitUntil(
      async () => (await el.getAttribute('data-block-state')) === 'EDITING',
      { timeout: 5000, timeoutMsg: 'block did not enter EDITING' },
    );
    await browser.pause(300);
  });

  it('step 1 — validation#s10-tag-invalid-char: UI 経由で禁止文字タグが reject されエラー表示（S10 の核）', async () => {
    const before = readMd();

    await addTag('foo,bar');

    const err = await $('[data-testid="screen-1-block-tag-error"]');
    await err.waitForExist({ timeout: 5000, timeoutMsg: 'tag error message did not appear' });
    expect(await err.getText()).toContain('invalid characters');

    // Note::assign_tag 非到達 → チップ追加なし / .md 不変 (永続化なし)
    expect(await chipTexts()).toEqual([]);
    const after = readMd();
    expect(extractTags(after)).toEqual([]);
    expect(extractUpdatedAt(after)).toBe(T0);
    expect(after).toBe(before);
  });

  it('step 2 — validation#s10-tag-invalid-char: Tauri 境界で禁止文字集合が invalid_tag で reject される', async () => {
    const before = readMd();

    for (const raw of FORBIDDEN_TAGS) {
      const res = await invokeAssignTag(raw);
      expect(res.ok).toBe(false);
      if (!res.ok) {
        expect(res.error.kind).toBe('invalid_tag');
        expect(res.error.reason ?? '').toContain('invalid character');
      }
    }

    // 空文字 (TagError::Empty) も InvalidTag として reject される
    const empty = await invokeAssignTag('   ');
    expect(empty.ok).toBe(false);
    if (!empty.ok) {
      expect(empty.error.kind).toBe('invalid_tag');
    }

    // reject 経路では永続化されない (Note::assign_tag 非到達 / event 非発行)
    const after = readMd();
    expect(after).toBe(before);
    expect(extractTags(after)).toEqual([]);
    expect(extractUpdatedAt(after)).toBe(T0);
  });

  it('step 3 — validation#s10-tag-invalid-char: 対照 — 正常タグは assign され永続化される', async () => {
    const before = readMd();
    expect(extractUpdatedAt(before)).toBe(T0);

    await addTag('valid');
    await browser.waitUntil(
      () => {
        try {
          return extractTags(readMd()).includes('valid');
        } catch {
          return false;
        }
      },
      { timeout: 3000, interval: 30, timeoutMsg: 'valid tag was not persisted' },
    );

    const after = readMd();
    expect(extractTags(after)).toEqual(['valid']);
    expect(extractUpdatedAt(after)).not.toBe(T0);

    await browser.waitUntil(async () => (await chipTexts()).includes('valid'), {
      timeout: 3000,
      timeoutMsg: 'valid chip did not appear in UI',
    });
  });
});
