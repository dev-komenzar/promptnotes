// @ori-generated scenario:s23-update-same-version-once
//
// 検証対象: validation.md#s23-update-same-version-once
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_* を per-run temp に隔離し、storage_dir を temp に向けた settings.json を seed する。
//        更新確認 endpoint は debug seam (TAURI_TEST_UPDATER_ENDPOINT) で 127.0.0.1 の local server に向け、
//        version ファイル (9.9.8 / 9.9.9) で「見つかるバージョン」を切り替える。
//
// 核: 同一プロセス内で通知済みのバージョンは、確認のたびに見つかっても再通知しない (I-U4)。
//     より新しいバージョンは通知する。
//
// バージョンの読み替え (spec.md#impl-notes): domain 例の current 0.2.2 / 通知済み 0.2.3 / 新版 0.2.4 を、
//   test build の current (ビルド時 version) < 9.9.8 < 9.9.9 として検証する。
// 24h ごとの定期確認は待たず、同じ確認処理 (check_for_updates) を Tauri 境界 invoke で実行する
//   (通知済み state は command 経路と定期 task 経路で共有、C-CFU12。周期判定は slice unit test TP-P*)。
// 再起動後の再通知 (Then 3) は E2E では扱わず slice unit test TP-D4 で代替担保する。

import { writeFileSync } from 'node:fs';

const VERSION_FILE = process.env.TAURI_TEST_S23_UPDATER_VERSION_FILE!;

const TOAST = '[data-testid="widget.widget-update-toast.root"]';
const LATEST = '[data-testid="widget.widget-update-toast.latest-version"]';
const DISMISS = '[data-testid="widget.widget-update-toast.dismiss"]';

type ReleaseResponse = { version: string; url: string; notes: string };
type UpdateChannelResponse = {
  current_version: string;
  latest_release: ReleaseResponse | null;
};

function setRemoteVersion(version: string): void {
  writeFileSync(VERSION_FILE, version);
}

async function runCheck(): Promise<UpdateChannelResponse> {
  const result = await browser.tauri.execute(async (tauri) => {
    return await tauri.core.invoke('check_for_updates');
  });
  return result as UpdateChannelResponse;
}

describe('scenario:s23-update-same-version-once', () => {
  it('given — validation#s23-given: 起動時の確認で 9.9.8 を通知し、ユーザが dismiss する', async () => {
    // Given: 起動時に page-main が listen 確立後 check_for_updates を自動 invoke → 9.9.8 の Toast
    const toast = await $(TOAST);
    await toast.waitForExist({ timeout: 15000 });
    expect(await $(LATEST).getText()).toBe('9.9.8');

    // Given: ユーザは Toast を dismiss 済み (アプリは終了していない)
    await $(DISMISS).click();
    await toast.waitForExist({ timeout: 5000, reverse: true });
    expect(await $(TOAST).isExisting()).toBe(false);
  });

  it('step 1 — validation#s23-then: 定期確認で再び 9.9.8 を検出しても通知済みのため Toast は再表示されない', async () => {
    // When 1: 前回確認から 24h 後の定期確認相当 — 同じ 9.9.8 が見つかる
    setRemoteVersion('9.9.8');
    const res = await runCheck();

    // Then: latest_release = Some(9.9.8) だが通知済みのため NewVersionDetected は発行されない (I-U4)
    expect(res.latest_release).not.toBeNull();
    expect(res.latest_release?.version).toBe('9.9.8');
    // emit → listen → render の遅延を吸収してから不在を確認する
    await browser.pause(500);
    expect(await $(TOAST).isExisting()).toBe(false);
  });

  it('step 2 — validation#s23-then: さらに後の定期確認で 9.9.9 を検出すると通知し Toast を表示する', async () => {
    // When 2: さらに 24h 後の定期確認相当 — 未通知の 9.9.9 が見つかる
    setRemoteVersion('9.9.9');
    const res = await runCheck();

    // Then: 9.9.9 は未通知のため NewVersionDetected { latest_version: 9.9.9 } → Toast 表示
    expect(res.latest_release?.version).toBe('9.9.9');
    const toast = await $(TOAST);
    await toast.waitForExist({ timeout: 10000 });
    expect(await $(LATEST).getText()).toBe('9.9.9');
  });
});
