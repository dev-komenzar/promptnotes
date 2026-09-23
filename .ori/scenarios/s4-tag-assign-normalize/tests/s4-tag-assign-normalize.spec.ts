// @ori-generated scenario:s4-tag-assign-normalize
//
// S4: タグ付与の正規化と重複排除
//
// runner: wdio (Tauri local mode, @wdio/tauri-service + tauri-driver)
// seed: wdio.conf.ts onPrepare が Note A (20260620120000, body="hello", tags=["gpt"]) を
//       TAURI_TEST_STORAGE_DIR に投入する
//
// domain/validation.md#s4-tag-assign-normalize:
//   - Tag 正規化: "  GPT  " → "gpt" (trim + lowercase)
//   - 正規化後 TagSet に既存 → assign は no-op (永続化・event なし, I-N5)
//   - 変化した場合のみ NoteTagsChanged 発行

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTE_ID = '20260620120000';

function readNote(): string {
  return readFileSync(join(process.env.TAURI_TEST_STORAGE_DIR!, `${NOTE_ID}.md`), 'utf-8');
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

describe('scenario:s4-tag-assign-normalize', () => {
  before(async () => {
    const el = await block();
    await el.waitForExist({ timeout: 10000 });
    await $('[data-testid="screen-1-block-tag-chip"]').waitForExist({ timeout: 10000 });
    await el.click();
    await browser.waitUntil(
      async () => (await el.getAttribute('data-block-state')) === 'EDITING',
      { timeout: 5000, timeoutMsg: 'block did not enter EDITING' },
    );
    await browser.pause(300);
  });

  it('step 1 — validation#s4-tag-assign-normalize: 正規化後重複するタグの assign は no-op', async () => {
    const before = readNote();
    expect(extractTags(before)).toEqual(['gpt']);
    const beforeUpdatedAt = extractUpdatedAt(before);

    await addTag('  GPT  ');
    await browser.pause(600);

    expect(await chipTexts()).toEqual(['gpt']);
    const after = readNote();
    expect(extractTags(after)).toEqual(['gpt']);
    expect(extractUpdatedAt(after)).toBe(beforeUpdatedAt);
  });

  it('step 2 — validation#s4-tag-assign-normalize: 新規タグ assign で永続化される', async () => {
    const before = readNote();
    const beforeUpdatedAt = extractUpdatedAt(before);

    await addTag('coding');
    await browser.waitUntil(
      () => {
        try {
          return extractTags(readNote()).includes('coding');
        } catch {
          return false;
        }
      },
      { timeout: 3000, interval: 30, timeoutMsg: 'new tag was not persisted' },
    );

    const after = readNote();
    expect(extractTags(after)).toEqual(['gpt', 'coding']);
    expect(extractUpdatedAt(after)).not.toBe(beforeUpdatedAt);

    await browser.waitUntil(async () => (await chipTexts()).includes('coding'), {
      timeout: 3000,
      timeoutMsg: 'coding chip did not appear in UI',
    });
  });
});
