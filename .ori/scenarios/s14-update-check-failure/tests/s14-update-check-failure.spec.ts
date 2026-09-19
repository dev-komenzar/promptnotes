// @ori-generated scenario:s14-update-check-failure
//
// 検証対象: validation.md#s14-update-check-failure
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_CONFIG_HOME/XDG_DATA_HOME と storage_dir を隔離し Note を seed する。
//        更新チェック endpoint は debug seam (TAURI_TEST_UPDATER_ENDPOINT) で 127.0.0.1 の
//        local server に向け、mode ファイルで fail (接続断) / release (有効な latest.json) を切替える。
//
// 核: 起動時の更新チェックが HTTP 失敗したとき silent に握り潰され (C-CFU1)、NewVersionDetected は
//     発行されない (I-U3) → 更新通知 Toast が mount されず UI 通知なし（ログのみ）。ユーザの作業を
//     妨げない。step 4 の陽性対照（成功時は Toast mount）により step 1-2 の失敗 inject が空振りで
//     ないことを示す。
//
// endpoint 差替の詳細 (spec.md#impl-notes / notes.md#test-approach):
//   production の release build は endpoint override を compile out するため、本 seam は test build
//   (debug) でのみ有効。`check_for_updates` は Tauri 境界 invoke (browser.tauri.execute) で実行する。

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_S14_NOTES_DIR!;
const MODE_FILE = process.env.TAURI_TEST_S14_UPDATER_MODE_FILE!;

const SEED = '20260620120000';

type ReleaseResponse = { version: string; url: string; notes: string };
type UpdateChannelResponse = {
  current_version: string;
  latest_release: ReleaseResponse | null;
};

function setMode(mode: 'fail' | 'release'): void {
  writeFileSync(MODE_FILE, mode);
}

function block(id: string) {
  return $(`[data-block-id="${id}"]`);
}

function readNote(id: string): string {
  return readFileSync(join(NOTES_DIR, `${id}.md`), 'utf-8');
}

async function checkForUpdates(): Promise<UpdateChannelResponse> {
  const result = await browser.tauri.execute(async (tauri) => {
    return await tauri.core.invoke('check_for_updates');
  });
  return result as UpdateChannelResponse;
}

describe('scenario:s14-update-check-failure', () => {
  before(async () => {
    // Given: ネットワーク断相当 (mode=fail) でアプリ起動直後の状態にする。
    setMode('fail');
    await (await block(SEED)).waitForExist({ timeout: 15000 });
    await browser.pause(500);
  });

  it('step 1 — validation#s14-update-check-failure: 更新チェックの HTTP 失敗は reject せず silent に latest_release=null へ降格する', async () => {
    // When: アプリ起動直後 (ネットワーク断) に check_for_updates を Tauri 境界で実行
    const res = await checkForUpdates();

    // Then: Result を露出せず (C-CFU1)、失敗は latest_release=null に正規化される (S14 / I-U2)。
    expect(res.current_version).toBe('0.2.1');
    expect(res.latest_release).toBeNull();
  });

  it('step 2 — validation#s14-update-check-failure: NewVersionDetected 非発行 → 更新通知 Toast は表示されない', async () => {
    // Then: event 非発行のため widget-update-toast は DOM に存在しない (I-U3 / screen-3 / S14 silent)。
    expect(await $('[data-testid="widget-update-toast"]').isExisting()).toBe(false);
  });

  it('step 3 — validation#s14-update-check-failure: 失敗してもユーザの作業を妨げない（フィードは生存）', async () => {
    // Then: seed 済み Note はフィードに表示され続け、永続化ファイルも無傷。
    expect(await (await block(SEED)).isExisting()).toBe(true);
    expect(readNote(SEED)).toContain('s14 seed body');
  });

  it('step 4 — validation#s14-update-check-failure: 陽性対照 — 更新チェック成功時は NewVersionDetected → Toast が mount される', async () => {
    // Given: endpoint を有効な release を返すモードへ切替
    setMode('release');

    // When: 同じ Tauri 境界 invoke を実行
    const res = await checkForUpdates();

    // Then: latest_release が Some (version > current) になり、event → Toast が mount される。
    //   これが step 1-2 の失敗 inject の非空虚性を保証する。
    expect(res.latest_release).not.toBeNull();
    expect(res.latest_release?.version).toBe('9.9.9');

    const toast = await $('[data-testid="widget-update-toast"]');
    await toast.waitForExist({ timeout: 10000 });
    expect(await $('[data-testid="screen-3-latest-version"]').getText()).toBe('9.9.9');
    expect(await $('[data-testid="screen-3-current-version"]').getText()).toContain('0.2.1');
  });
});
