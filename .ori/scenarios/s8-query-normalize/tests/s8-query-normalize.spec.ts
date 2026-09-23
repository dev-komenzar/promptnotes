// @ori-generated scenario:s8-query-normalize
//
// 検証対象: validation.md#s8-query-normalize
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が TAURI_TEST_STORAGE_DIR に
//        Note A (20260620120000, body="GPT を試す") /
//        Note B (20260620130000, body="ｇｐｔ のメモ" 全角) を seed する
//
// 核: 検索バーに半角 "gpt" を入力したとき、A（半角 body）だけでなく
//     B（全角 body）も NFKC 互換等価変換で match して **両方** 表示されること。

import { existsSync } from 'node:fs';
import { join } from 'node:path';

function notePath(storageDir: string, id: string): string {
  return join(storageDir, `${id}.md`);
}

async function count(selector: string): Promise<number> {
  const els = await $$(selector);
  return (els as unknown as { length: number }).length;
}

type FilterDto = {
  query: string | null;
  date_range: unknown;
  tag: string | null;
};

type InvokeResult = { ok: true; value: FilterDto } | { ok: false; error: unknown };

/**
 * `update_feed_filter` Tauri command をブラウザ文脈から直接 invoke し、
 * 返却 DTO (`NoteFeedFilterDto`) の query 正規化を観測する。
 *
 * `@wdio/tauri-service` (driverProvider=external) の patchedExecute は
 * `browser.executeAsync` を扱えないため、同期 `browser.execute` で kick-off し
 * window 上の結果を poll する (S7 と同方式)。poll は WebKitWebDriver の
 * serialization 制約を避けるため**常に文字列**を返す。
 */
async function invokeUpdateFeedFilter(raw: string): Promise<FilterDto> {
  await browser.execute((rawQuery: string) => {
    const w = window as unknown as {
      __TAURI_INTERNALS__?: {
        invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
      __oriS8Filter?: string;
    };
    w.__oriS8Filter = 'pending';
    const invoke = w.__TAURI_INTERNALS__?.invoke;
    if (!invoke) {
      w.__oriS8Filter = JSON.stringify({ ok: false, error: { kind: 'no_tauri_internals' } });
      return;
    }
    invoke('update_feed_filter', { input: { kind: 'set_query', raw: rawQuery } })
      .then((value) => {
        w.__oriS8Filter = JSON.stringify({ ok: true, value });
      })
      .catch((error: unknown) => {
        w.__oriS8Filter = JSON.stringify({ ok: false, error });
      });
  }, raw);

  for (let i = 0; i < 50; i++) {
    const observed = await browser.execute((key: string) => {
      const store = window as unknown as Record<string, unknown>;
      const value = store[key];
      return typeof value === 'string' ? value : 'pending';
    }, '__oriS8Filter');
    if (observed !== 'pending') {
      const parsed = JSON.parse(observed) as InvokeResult;
      if (!parsed.ok) throw new Error(`update_feed_filter rejected: ${JSON.stringify(parsed.error)}`);
      return parsed.value;
    }
    await browser.pause(100);
  }
  throw new Error('update_feed_filter result was not observed (S8)');
}

describe('scenario:s8-query-normalize', () => {
  const STORAGE_DIR = process.env.TAURI_TEST_STORAGE_DIR!;
  const A = '20260620120000';
  const B = '20260620130000';
  const PATH_A = notePath(STORAGE_DIR, A);
  const PATH_B = notePath(STORAGE_DIR, B);

  const block = (id: string) => `[data-block-id="${id}"]`;
  const ALL_BLOCKS = '[data-testid="screen-1-block"]';
  const SEARCH = '[data-testid="screen-1-toolbar-search-query"]';

  before(async () => {
    await browser.waitUntil(async () => (await count(ALL_BLOCKS)) >= 2, {
      timeout: 10000,
      timeoutMsg: 'seeded notes A/B did not appear in feed',
    });
    await browser.pause(1200);
  });

  it('step 1 — validation#s8-query-normalize: 半角 query で全角 body も NFKC match する', async () => {
    // ---- Given: A / B が表示され、検索バーは空 ----
    expect(await count(ALL_BLOCKS)).toBe(2);
    expect(existsSync(PATH_A)).toBe(true);
    expect(existsSync(PATH_B)).toBe(true);

    const search = await $(SEARCH);
    await search.waitForExist({ timeout: 5000 });
    expect(await search.getValue()).toBe('');

    // ---- When: 半角 "gpt" を入力 ----
    await search.setValue('gpt');
    await browser.pause(800);

    // ---- Then: A (半角 body) と B (全角 body, NFKC) が両方 match ----
    expect(await count(block(A))).toBe(1);
    expect(await count(block(B))).toBe(1);
    expect(await count(ALL_BLOCKS)).toBe(2);

    // filter.query は NFKC + lowercase 済み ("gpt" は変化なし)
    const dto = await invokeUpdateFeedFilter('gpt');
    expect(dto.query).toBe('gpt');
  });

  it('step 2 — validation#s8-query-normalize: 全角大文字 query を正規化しても両方表示', async () => {
    // ---- When: 全角大文字 "ＧＰＴ" を入力 ----
    const search = await $(SEARCH);
    await search.setValue('ＧＰＴ');
    await browser.pause(800);

    // ---- Then: query 正規化 + body 正規化の合成で両方 match ----
    expect(await count(block(A))).toBe(1);
    expect(await count(block(B))).toBe(1);
    expect(await count(ALL_BLOCKS)).toBe(2);

    // Rust 境界の NormalizedQuery が全角大文字 → "gpt" へ正規化する
    const dto = await invokeUpdateFeedFilter('ＧＰＴ');
    expect(dto.query).toBe('gpt');
  });

  it('step 3 — validation#s8-query-normalize: query 解除で両方再表示', async () => {
    // ---- When: 検索バーを空に戻す ----
    const search = await $(SEARCH);
    await search.clearValue();
    await browser.pause(800);

    // ---- Then: filter 解除、A / B が再表示 ----
    expect(await count(block(A))).toBe(1);
    expect(await count(block(B))).toBe(1);
    expect(await count(ALL_BLOCKS)).toBe(2);
    expect(await search.getValue()).toBe('');
  });
});
