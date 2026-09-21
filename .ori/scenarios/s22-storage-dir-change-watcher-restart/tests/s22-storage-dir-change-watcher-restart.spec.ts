// @ori-generated scenario:s22-storage-dir-change-watcher-restart
//
// 検証対象: validation.md#s22-storage-dir-change-watcher-restart
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME / XDG_DATA_HOME を隔離し、settings.json
//        (storage_dir = old dir) と old dir 3 件 / new dir 1 件を seed する。
//        app は起動時に PageMain から start_file_watcher を invoke し old dir を監視する。
//
// 核: 設定で storage_dir を new dir へ変更したとき、StorageDirChanged subscriber (infrastructure) が
//     旧 watcher を停止し new dir で watcher を起動し直すこと。判定は
//     「StorageDirChanged 後に new dir の外部変更が手動操作なしで UI に反映されること」で行う
//     (domain event は frontend に露出しないため。spec.md#impl-notes)。
//
// 注意: 本 scenario の step 3 / 4 は watcher 再起動が未実装のため RED になるのが想定である
//     (TP3 / TP4)。RED である事実と原因は review.md / notes.md に記録する。

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const OLD_DIR = process.env.TAURI_TEST_S22_OLD_DIR!;
const NEW_DIR = process.env.TAURI_TEST_S22_NEW_DIR!;
const SETTINGS_PATH = process.env.TAURI_TEST_S22_SETTINGS!;

const OLD_IDS = ['20260601100000', '20260615100000', '20260626100000'];
const NEW_ID = '20260701100000';
const NEW_CREATED_ID = '20260702100000';

const OLD_A_EXTERNAL_BODY = 'old-alpha external baseline';
const NEW_DELTA_EXTERNAL_BODY = 'new-delta detected after storage_dir change';
const NEW_CREATED_BODY = 'new-created detected after storage_dir change';

type PersistedSettings = { storage_dir: string; theme: string };

function readSettings(): PersistedSettings {
  return JSON.parse(readFileSync(SETTINGS_PATH, 'utf-8')) as PersistedSettings;
}

function noteMd(id: string, body: string): string {
  return ['---', `createdAt: ${id}`, `updatedAt: ${id}`, 'tags: []', '---', body].join('\n');
}

function writeExternalNote(dir: string, id: string, body: string): void {
  writeFileSync(join(dir, `${id}.md`), noteMd(id, body), 'utf-8');
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

async function blockBodyText(id: string): Promise<string> {
  const content = await $(`[data-block-id="${id}"] .cm-content`);
  if (!(await content.isExisting())) return '';
  return (await content.getText()).trim();
}

async function waitForBlockBody(id: string, expected: string, timeoutMsg: string): Promise<void> {
  await browser.waitUntil(async () => (await blockBodyText(id)) === expected, {
    timeout: 15_000,
    interval: 200,
    timeoutMsg,
  });
}

type UpdateSettingsResult =
  | { ok: true; value: { storage_dir: string } }
  | { ok: false; error: unknown };

/**
 * `update_settings` Tauri command をブラウザ文脈から直接 invoke する。同期 `browser.execute` で
 * kick-off し window 上の結果を poll する (`@wdio/tauri-service` (driverProvider=external) の
 * patchedExecute は browser.executeAsync を扱えないため。s10 / s11 と同方式)。
 */
async function invokeUpdateSettings(input: Record<string, unknown>): Promise<UpdateSettingsResult> {
  await browser.execute(
    (payload: Record<string, unknown>) => {
      const w = window as unknown as {
        __TAURI_INTERNALS__?: {
          invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
        __oriS22UpdateSettings?: string;
      };
      w.__oriS22UpdateSettings = 'pending';
      const invoke = w.__TAURI_INTERNALS__?.invoke;
      if (!invoke) {
        w.__oriS22UpdateSettings = JSON.stringify({ ok: false, error: { kind: 'no_tauri_internals' } });
        return;
      }
      invoke('update_settings', { input: payload })
        .then((value) => {
          w.__oriS22UpdateSettings = JSON.stringify({ ok: true, value });
        })
        .catch((error: unknown) => {
          w.__oriS22UpdateSettings = JSON.stringify({ ok: false, error });
        });
    },
    input,
  );

  for (let i = 0; i < 50; i++) {
    const observed = await browser.execute((key: string) => {
      const store = window as unknown as Record<string, unknown>;
      return typeof store[key] === 'string' ? (store[key] as string) : 'pending';
    }, '__oriS22UpdateSettings');
    if (observed !== 'pending') {
      return JSON.parse(observed) as UpdateSettingsResult;
    }
    await browser.pause(100);
  }
  throw new Error('update_settings result was not observed (S22)');
}

describe('scenario:s22-storage-dir-change-watcher-restart', () => {
  before(async () => {
    // Given: old dir の 3 件が表示されるまで待つ（app 起動時に start_file_watcher が old dir を監視）
    await browser.waitUntil(
      async () => {
        const ids = await blockIds();
        return OLD_IDS.every((id) => ids.includes(id));
      },
      { timeout: 15_000, timeoutMsg: 'seeded old-dir notes did not appear in feed' },
    );
    expect([...OLD_IDS].sort()).toEqual((await blockIds()).slice().sort());
    expect(readSettings().storage_dir).toBe(OLD_DIR);
  });

  it('step 1 — validation#s22-storage-dir-change-watcher-restart: Given 前提として old dir の watcher が稼働している', async () => {
    // When: 外部プログラムが old dir の Note を上書きする
    writeExternalNote(OLD_DIR, OLD_IDS[0], OLD_A_EXTERNAL_BODY);

    // Then: 手動操作なしで old dir の変更が DOM に自動反映される（watcher 稼働の前提実測）
    await waitForBlockBody(
      OLD_IDS[0],
      OLD_A_EXTERNAL_BODY,
      'old dir watcher が稼働していない (Given 前提が崩れている)',
    );

    // Then: storage_dir はまだ変更していないので再起動モーダルは出ない
    expect(await $('[data-testid="restart-prompt"]').isExisting()).toBe(false);
  });

  it('step 2 — validation#s22-storage-dir-change-watcher-restart: storage_dir 変更で永続化 + 再起動モーダル + フィードは旧 dir のまま (S11 回帰)', async () => {
    // When: storage_dir を new dir に変更する（native folder picker を駆動できないため境界 invoke）
    const res = await invokeUpdateSettings({ storage_dir: NEW_DIR });
    expect(res.ok).toBe(true);
    if (res.ok) expect(res.value.storage_dir).toBe(NEW_DIR);

    // Then: app_config_dir/settings.json の storage_dir が new dir に更新される
    await browser.waitUntil(() => readSettings().storage_dir === NEW_DIR, {
      timeout: 5_000,
      timeoutMsg: 'settings.json storage_dir was not updated',
    });

    // Then: 再起動を促すモーダルが表示される (I-S4 / StorageDirChanged subscriber)
    await $('[data-testid="restart-prompt"]').waitForExist({
      timeout: 5_000,
      timeoutMsg: 'restart prompt did not appear after storage_dir change',
    });

    // Then: 変更直後のフィードは旧ディレクトリのまま (new dir の Note は現れない。I-S4 回帰)
    expect((await blockIds()).slice().sort()).toEqual([...OLD_IDS].sort());
    expect(await blockIds()).not.toContain(NEW_ID);
  });

  it('step 3 — validation#s22-storage-dir-change-watcher-restart: StorageDirChanged 後に new dir の外部変更が検知される (核)', async () => {
    // When: 変更後、外部プログラムが new dir の既存 Note を上書きする
    writeExternalNote(NEW_DIR, NEW_ID, NEW_DELTA_EXTERNAL_BODY);

    // Then: 手動 Refresh / 再起動なしで UI が new dir の変更を反映する
    //       (= infrastructure subscriber が旧 watcher を停止し new dir で watcher を起動した)
    await waitForBlockBody(
      NEW_ID,
      NEW_DELTA_EXTERNAL_BODY,
      'storage_dir 変更後に new dir の変更が検知されなかった (StorageDirChanged の infrastructure subscriber = watcher 再起動が未実装)',
    );

    // Then: フィードは new dir の内容に切り替わっている（old dir の Note は消える）
    expect((await blockIds()).slice().sort()).toEqual([NEW_ID]);
  });

  it('step 4 — validation#s22-storage-dir-change-watcher-restart: StorageDirChanged 後に new dir の新規 .md が検知される (Created 経路)', async () => {
    // When: 変更後、外部プログラムが new dir に新しい Note を作成する
    writeExternalNote(NEW_DIR, NEW_CREATED_ID, NEW_CREATED_BODY);

    // Then: 手動操作なしで新規 Block が出現し body が反映される
    await browser.waitUntil(
      async () => (await blockIds()).includes(NEW_CREATED_ID),
      {
        timeout: 15_000,
        interval: 200,
        timeoutMsg:
          'storage_dir 変更後に new dir の新規 .md が検知されなかった (new dir watcher の Created 経路が未稼働)',
      },
    );
    await waitForBlockBody(
      NEW_CREATED_ID,
      NEW_CREATED_BODY,
      'new dir の新規 Note の body が反映されていない',
    );
  });
});
