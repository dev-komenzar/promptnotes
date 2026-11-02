// @ori-generated scenario:s26-unbundled-notify-only
//
// 検証対象: validation.md#s26-unbundled-notify-only
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_* を per-run temp に隔離し、storage_dir を temp に向けた settings.json を seed する。
//        更新確認は 127.0.0.1 の local server (debug seam TAURI_TEST_UPDATER_ENDPOINT) が 9.9.8 を返す。
//        配布形式は `--no-bundle` の test build そのもの = InstallationKind::Unbundled (seam で上書きしない)。
//
// 核: Unbundled ではインストール操作を提示せず通知のみ (View release / dismiss)。
//     install が要求されても UnsupportedInstallation で拒否し、ネットワークには触れない (I-U6)。

import { readFileSync } from 'node:fs';

const APPIMAGE = process.env.TAURI_TEST_S26_APPIMAGE!;
const REQUEST_COUNTS = process.env.TAURI_TEST_S26_REQUEST_COUNTS!;

const TOAST = '[data-testid="widget.widget-update-toast.root"]';
const LATEST = '[data-testid="widget.widget-update-toast.latest-version"]';
const INSTALL = '[data-testid="widget.widget-update-toast.install-update"]';
const STATUS = '[data-testid="widget.widget-update-toast.install-status"]';
const VIEW_RELEASE = '[data-testid="widget.widget-update-toast.view-release"]';
const DISMISS = '[data-testid="widget.widget-update-toast.dismiss"]';

type InstallResult = { ok: true; version: string } | { ok: false; kind: string; message: string };

function requestCounts(): Record<string, number> {
  return JSON.parse(readFileSync(REQUEST_COUNTS, 'utf-8')) as Record<string, number>;
}

describe('scenario:s26-unbundled-notify-only', () => {
  it('given — validation#s26-given: Unbundled のビルドで、起動時の確認が新バージョンを検出する', async () => {
    // Given: 起動時の確認で 9.9.8 を検出 (endpoint へ取得に来ている)
    const toast = await $(TOAST);
    await toast.waitForExist({ timeout: 15000 });
    expect(requestCounts()['/latest.json'] ?? 0).toBeGreaterThanOrEqual(1);

    // Given: 配布形式は Unbundled (is_update_installable == false)
    const installable = await browser.tauri.execute(async (tauri) => {
      return await tauri.core.invoke('is_update_installable');
    });
    expect(installable).toBe(false);
  });

  it('step 1 — validation#s26-then: Toast は View release と dismiss のみで、「Update & restart」を表示しない', async () => {
    // When: Toast が表示される
    expect(await $(LATEST).getText()).toBe('9.9.8');

    // Then: is_installable() == false のため「Update & restart」は表示されない (I-U6)
    expect(await $(INSTALL).isExisting()).toBe(false);
    expect(await $(STATUS).isExisting()).toBe(false);
    // Then: View release と dismiss は使える
    expect(await $(VIEW_RELEASE).isDisplayed()).toBe(true);
    expect(await $(VIEW_RELEASE).isEnabled()).toBe(true);
    expect(await $(DISMISS).isDisplayed()).toBe(true);
    expect(await $(DISMISS).isEnabled()).toBe(true);
  });

  it('step 2 — validation#s26-then: install が要求されても UnsupportedInstallation で拒否し、ネットワークには触れない', async () => {
    const before = requestCounts();
    const appImageBefore = readFileSync(APPIMAGE);

    // When: (UI からは出せないが) install が要求される — Tauri 境界で install_update を invoke する
    const result = (await browser.tauri.execute(async (tauri) => {
      try {
        const version = await tauri.core.invoke('install_update');
        return { ok: true, version: String(version) };
      } catch (e) {
        const err = e as { kind?: string; message?: string };
        return { ok: false, kind: String(err.kind), message: String(err.message) };
      }
    })) as InstallResult;

    // Then: UnsupportedInstallation で拒否する
    expect(result.ok).toBe(false);
    expect(result.ok ? '' : result.kind).toBe('UnsupportedInstallation');
    // Then: ネットワークには触れない (最新リリースの取得し直しも download も起きない)。何もインストールされない
    expect(requestCounts()).toEqual(before);
    expect(readFileSync(APPIMAGE)).toEqual(appImageBefore);
    // Then: Toast は通知のまま残る
    expect(await $(TOAST).isExisting()).toBe(true);
  });
});
