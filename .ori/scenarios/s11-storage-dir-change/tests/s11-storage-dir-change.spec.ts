// @ori-generated scenario:s11-storage-dir-change
//
// 検証対象: validation.md#s11-storage-dir-change
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME/XDG_DATA_HOME を隔離し、
//        settings.json (storage_dir = old dir) と old dir 3 件 / new dir 1 件を seed する
//
// 核: storage_dir 変更は即時マイグレーションしない (I-S4)。update-settings workflow は
//     settings.json へ永続化し StorageDirChanged を発行する。UI は再起動モーダルを表示し、
//     Feed は再起動まで旧ディレクトリを見続ける。再起動後のみ new dir のスキャン結果を表示する。
//
// 保存ディレクトリは read-only + OS native folder picker のため dialog を駆動できない。そこで
// storage_dir 変更は `update_settings` を Tauri 境界で直接 invoke し (s10 の境界 invoke と同方式)、
// 設定モーダルの Save 経路 (handleSettingsSaved) は theme 変更の UI save で踏む。storage_dir 変更を
// modal open 中に起こすことで、Save 時の storage_dir 差分に対する Feed 据え置き guard も検証できる。

import { readFileSync } from 'node:fs';

const OLD_DIR = process.env.TAURI_TEST_S11_OLD_DIR!;
const NEW_DIR = process.env.TAURI_TEST_S11_NEW_DIR!;
const SETTINGS_PATH = process.env.TAURI_TEST_S11_SETTINGS!;

const OLD_IDS = ['20260601100000', '20260615100000', '20260626100000'];
const NEW_ID = '20260701100000';

type PersistedSettings = { storage_dir: string; theme: string };

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

type UpdateSettingsResult = { ok: true; value: { storage_dir: string } } | { ok: false; error: unknown };

/**
 * `update_settings` Tauri command をブラウザ文脈から直接 invoke する。同期 `browser.execute` で
 * kick-off し window 上の結果を poll する (`@wdio/tauri-service` (driverProvider=external) の
 * patchedExecute は browser.executeAsync を扱えないため。s10 と同方式)。
 */
async function invokeUpdateSettings(input: Record<string, unknown>): Promise<UpdateSettingsResult> {
  await browser.execute(
    (payload: Record<string, unknown>) => {
      const w = window as unknown as {
        __TAURI_INTERNALS__?: {
          invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
        __oriS11UpdateSettings?: string;
      };
      w.__oriS11UpdateSettings = 'pending';
      const invoke = w.__TAURI_INTERNALS__?.invoke;
      if (!invoke) {
        w.__oriS11UpdateSettings = JSON.stringify({ ok: false, error: { kind: 'no_tauri_internals' } });
        return;
      }
      invoke('update_settings', { input: payload })
        .then((value) => {
          w.__oriS11UpdateSettings = JSON.stringify({ ok: true, value });
        })
        .catch((error: unknown) => {
          w.__oriS11UpdateSettings = JSON.stringify({ ok: false, error });
        });
    },
    input,
  );

  for (let i = 0; i < 50; i++) {
    const observed = await browser.execute((key: string) => {
      const store = window as unknown as Record<string, unknown>;
      return typeof store[key] === 'string' ? (store[key] as string) : 'pending';
    }, '__oriS11UpdateSettings');
    if (observed !== 'pending') {
      return JSON.parse(observed) as UpdateSettingsResult;
    }
    await browser.pause(100);
  }
  throw new Error('update_settings result was not observed (S11)');
}

async function openSettingsModal() {
  await $('[data-testid="screen-1-toolbar-settings-button"]').click();
  const modal = await $('[data-testid="widget-settings-modal"]');
  await modal.waitForExist({ timeout: 5000, timeoutMsg: 'settings modal did not open' });
  return modal;
}

describe('scenario:s11-storage-dir-change', () => {
  before(async () => {
    // Given: old dir の 3 件が表示されるまで待つ
    await browser.waitUntil(
      async () => {
        const ids = await blockIds();
        return OLD_IDS.every((id) => ids.includes(id));
      },
      { timeout: 15000, timeoutMsg: 'seeded old-dir notes did not appear in feed' },
    );
    expect([...OLD_IDS].sort()).toEqual((await blockIds()).sort());
    expect(readSettings().storage_dir).toBe(OLD_DIR);
  });

  it('step 1 — validation#s11-storage-dir-change: 設定モーダルは old dir を表示し、theme の UI save は再起動モーダルを出さない', async () => {
    const modal = await openSettingsModal();

    // Then: 保存ディレクトリに Given の old dir が表示される
    const input = await $('[data-testid="screen-2-storage-dir"]');
    await input.waitForExist({ timeout: 5000 });
    expect(await input.getValue()).toBe(OLD_DIR);

    // When: 対照として theme のみを UI から変更して保存する
    await $('[data-testid="screen-2-theme-Dark"]').click();
    await $('[data-testid="screen-2-save"]').click();

    // Then: theme が永続化され modal は閉じ、再起動モーダルは出ない (StorageDirChanged 非発行)
    await modal.waitForExist({ timeout: 5000, reverse: true, timeoutMsg: 'settings modal did not close' });
    await browser.waitUntil(() => readSettings().theme === 'Dark', {
      timeout: 5000,
      timeoutMsg: 'theme was not persisted by the settings modal save',
    });
    expect(await $('[data-testid="restart-prompt"]').isExisting()).toBe(false);
    expect(readSettings().storage_dir).toBe(OLD_DIR);
    expect((await blockIds()).slice().sort()).toEqual([...OLD_IDS].sort());
  });

  it('step 2 — validation#s11-storage-dir-change: storage_dir 変更で永続化 + 再起動モーダル表示 + フィードは旧 dir のまま', async () => {
    const before = (await blockIds()).slice().sort();

    // Given: 設定モーダルを開いておく
    const modal = await openSettingsModal();

    // When: storage_dir を new dir に変更する (native folder picker を駆動できないため境界 invoke)
    const res = await invokeUpdateSettings({ storage_dir: NEW_DIR });
    expect(res.ok).toBe(true);
    if (res.ok) expect(res.value.storage_dir).toBe(NEW_DIR);

    // Then: app_config_dir/settings.json の storage_dir が new dir に更新される
    await browser.waitUntil(() => readSettings().storage_dir === NEW_DIR, {
      timeout: 5000,
      timeoutMsg: 'settings.json storage_dir was not updated',
    });

    // Then: 再起動を促すモーダルが表示される (I-S4 / StorageDirChanged subscriber)
    const prompt = await $('[data-testid="restart-prompt"]');
    await prompt.waitForExist({ timeout: 5000, timeoutMsg: 'restart prompt did not appear' });

    // When: さらに modal から theme を変更して保存する (storage_dir 差分を含む Save 経路)
    await $('[data-testid="screen-2-theme-Light"]').click();
    await $('[data-testid="screen-2-save"]').click();
    await modal.waitForExist({ timeout: 5000, reverse: true, timeoutMsg: 'settings modal did not close' });
    await browser.waitUntil(() => readSettings().theme === 'Light', {
      timeout: 5000,
      timeoutMsg: 'theme was not persisted by the settings modal save',
    });

    // Then: フィードは旧ディレクトリのまま (new dir の Note は現れない。I-S4)
    expect((await blockIds()).slice().sort()).toEqual(before);
    expect(await blockIds()).not.toContain(NEW_ID);
    expect(readSettings().storage_dir).toBe(NEW_DIR);
  });

  it('step 3 — validation#s11-storage-dir-change: 再起動後に new dir のスキャン結果が表示される', async () => {
    // When: 「今すぐ再起動」→ window.location.reload() で remount する
    await $('[data-testid="restart-prompt-restart"]').click();

    // Then: 再読込後は new dir の Note 1 件が表示される
    await browser.waitUntil(
      async () => (await blockIds()).includes(NEW_ID),
      { timeout: 15000, timeoutMsg: 'new-dir note did not appear after restart' },
    );

    expect((await blockIds()).slice().sort()).toEqual([NEW_ID]);
    expect(readSettings().storage_dir).toBe(NEW_DIR);
  });
});
