// @ori-generated scenario:s12-startup-state
//
// 検証対象: validation.md#s12-startup-state
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME/XDG_DATA_HOME を隔離し、
//        settings.json (sort_preference = { updated_at, asc }) と Note 3 件を seed する。
//        updatedAt 昇順 は createdAt 昇順 / createdAt 降順 (I-S3 default) のいずれとも異なる。
//
// 核: アプリ起動時、filter (query / date_range / tag) は **空** (挥発、Q3 / I-F6) で初期化され、
//     sort のみ Settings.sort_preference から復元される。結果として全 Note が updatedAt 昇順で
//     表示される。filter は永続化されないため settings.json には現れない (前回セッションの
//     query "gpt" / tag "coding" は持ち越されない)。
//
// E2E の新規セッション起動はプロセス新規起動であり、NoteFeed の default filter (空) から始まる
// = S12 の「再起動後」状態そのもの。したがって起動直後の filter / sort / 表示順を検証する。

import { readFileSync } from 'node:fs';

const NOTES_DIR = process.env.TAURI_TEST_S12_NOTES_DIR!;
const SETTINGS_PATH = process.env.TAURI_TEST_S12_SETTINGS!;

const ALPHA = '20260101100000'; // created 01-01 / updated 06-01
const BETA = '20260201100000'; // created 02-01 / updated 04-01
const GAMMA = '20260301100000'; // created 03-01 / updated 05-01

const UPDATED_AT_ASC = [BETA, GAMMA, ALPHA];
const CREATED_AT_ASC = [ALPHA, BETA, GAMMA];
const CREATED_AT_DESC = [GAMMA, BETA, ALPHA];

type PersistedSettings = {
  storage_dir: string;
  theme: string;
  sort_preference: { field: string; direction: string };
};

function readSettings(): PersistedSettings {
  return JSON.parse(readFileSync(SETTINGS_PATH, 'utf-8')) as PersistedSettings;
}

async function blockIds(): Promise<string[]> {
  try {
    const ids = await browser.execute(() => {
      const els = document.querySelectorAll('[data-block-id]');
      return Array.from(els).map((e) => e.getAttribute('data-block-id') ?? '');
    });
    return ids as unknown as string[];
  } catch {
    // page reload 中は execution context が落ちるため空扱いで poll を継続する
    return [];
  }
}

async function ariaPressed(testid: string): Promise<string | null> {
  return $('[data-testid="' + testid + '"]').getAttribute('aria-pressed');
}

describe('scenario:s12-startup-state', () => {
  before(async () => {
    // Given: seeded Note 3 件が表示されるまで待つ (load-settings → list-feed 完了)
    await browser.waitUntil(async () => (await blockIds()).length === 3, {
      timeout: 15000,
      timeoutMsg: 'seeded notes did not appear in feed',
    });
  });

  it('step 1 — validation#s12-startup-state: 起動時 filter は空、sort は Settings から復元され updatedAt 昇順で表示される', async () => {
    // Then: sort は Settings から復元され、updatedAt 昇順で表示される。
    // I-S3 default { created_at, desc } / createdAt 昇順 のいずれとも異なる順序であることを明示する。
    expect((await blockIds()).slice()).toEqual(UPDATED_AT_ASC);
    expect((await blockIds()).slice()).not.toEqual(CREATED_AT_DESC);
    expect((await blockIds()).slice()).not.toEqual(CREATED_AT_ASC);

    // Then: toolbar が復元後の sort (field=updated_at / direction=asc) を反映している
    expect(await ariaPressed('screen-1-toolbar-sort-field-updated_at')).toBe('true');
    expect(await $('[data-testid="screen-1-toolbar-sort-direction"]').getAttribute('aria-label')).toBe(
      'Ascending',
    );

    // Then: filter は空 (query なし / date_range=All / tag なし)。
    // 前回セッションの "gpt" / "coding" は挥発のため持ち越されない (Q3 / I-F6 / I-F2)。
    expect(await $('[data-testid="screen-1-toolbar-search-query"]').getValue()).toBe('');
    expect(await ariaPressed('screen-1-toolbar-date-range-all')).toBe('true');
    expect(await $('[data-testid="screen-1-toolbar-tag-chip"]').isExisting()).toBe(false);

    // Then: settings.json は seed 値のまま (起動が永続化を書き換えない)
    const settings = readSettings();
    expect(settings.storage_dir).toBe(NOTES_DIR);
    expect(settings.sort_preference).toEqual({ field: 'updated_at', direction: 'asc' });

    // Then: filter は Settings に永続化されない (挥発、Q3 / I-F6)
    expect(Object.keys(settings)).not.toContain('query');
    expect(Object.keys(settings)).not.toContain('tag');
    expect(Object.keys(settings)).not.toContain('date_range');
    expect(Object.keys(settings)).not.toContain('filter');
  });
});
