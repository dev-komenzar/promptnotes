// @ori-generated scenario:s25-install-update-failure
//
// 検証対象: validation.md#s25-install-update-failure
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_* を per-run temp に隔離し、storage_dir を temp に向けた settings.json を seed する。
//        更新配信は 127.0.0.1 の local server (debug seam TAURI_TEST_UPDATER_ENDPOINT)。署名検証の公開鍵は
//        onPrepare が実行時に生成した鍵 (debug seam TAURI_TEST_UPDATER_PUBKEY)。配布形式は debug seam
//        TAURI_TEST_INSTALLATION_KIND=deb。mode ファイルで失敗の種類 (download-fail / bad-signature) を切り替える。
//
// 核: インストールの失敗はユーザに伝え (silent にしない)、アプリは再起動せず継続し、
//     「Update & restart」を再び押せる状態に戻す。自動リトライはしない (I-U7)。
//
// 失敗の再現 (spec.md#impl-notes):
//   - DownloadFailed: download が HTTP 500 (validation の「ネットワークエラー」相当)
//   - InstallFailed: 署名検証失敗 (latest.json の署名が配信パッケージと一致しない)。
//     deb の権限昇格 (pkexec) の拒否は実 deb + 対話認証が要るため E2E では扱わない
//   - エラー種別は UI に出ないため、UI 操作の後に同じ install_update を Tauri 境界 invoke して種別を確認する

import { existsSync, readFileSync, writeFileSync } from 'node:fs';

const APPIMAGE = process.env.TAURI_TEST_S25_APPIMAGE!;
const MARKER = process.env.TAURI_TEST_S25_MARKER!;
const MODE_FILE = process.env.TAURI_TEST_S25_MODE_FILE!;
const REQUEST_COUNTS = process.env.TAURI_TEST_S25_REQUEST_COUNTS!;

const TOAST = '[data-testid="widget.widget-update-toast.root"]';
const LATEST = '[data-testid="widget.widget-update-toast.latest-version"]';
const INSTALL = '[data-testid="widget.widget-update-toast.install-update"]';
const STATUS = '[data-testid="widget.widget-update-toast.install-status"]';
const VIEW_RELEASE = '[data-testid="widget.widget-update-toast.view-release"]';
const DISMISS = '[data-testid="widget.widget-update-toast.dismiss"]';

const FAILED_TEXT = 'Update failed. Try again or download it from the release page.';

// 同じ webview / process が生きていることの目印 (reload や再起動で消える)。テストが window に置くだけで app は読まない
const SENTINEL = 's25-same-process';

type InstallResult = { ok: true; version: string } | { ok: false; kind: string; message: string };

async function readSentinel(): Promise<string | null> {
  return browser.execute(
    () => (window as unknown as { __s25Sentinel?: string }).__s25Sentinel ?? null,
  );
}

function setMode(mode: 'download-fail' | 'bad-signature'): void {
  writeFileSync(MODE_FILE, mode);
}

function requestCount(path: string): number {
  const counts = JSON.parse(readFileSync(REQUEST_COUNTS, 'utf-8')) as Record<string, number>;
  return counts[path] ?? 0;
}

/** install_update を Tauri 境界 invoke し、エラー種別 (InstallUpdateErrorResponse.kind) を取り出す。 */
async function invokeInstall(): Promise<InstallResult> {
  const result = await browser.tauri.execute(async (tauri) => {
    try {
      const version = await tauri.core.invoke('install_update');
      return { ok: true, version: String(version) };
    } catch (e) {
      const err = e as { kind?: string; message?: string };
      return { ok: false, kind: String(err.kind), message: String(err.message) };
    }
  });
  return result as InstallResult;
}

/** 「Update & restart」を押し、処理中表示を経て失敗表示に戻るまで待つ。 */
async function clickInstallAndWaitForFailure(): Promise<void> {
  await $(INSTALL).click();
  await browser.waitUntil(async () => (await $(STATUS).getText()) === 'Updating…', {
    timeout: 2000,
    timeoutMsg: 'install status did not become "Updating…"',
  });
  await browser.waitUntil(async () => (await $(STATUS).getText()) === FAILED_TEXT, {
    timeout: 15000,
    timeoutMsg: 'install status did not become the failure message',
  });
}

/** 失敗後の共通 Then: アプリは継続し、再起動もインストールもしておらず、再操作できる。 */
async function expectRecoveredFromFailure(appImageBefore: Buffer): Promise<void> {
  // Toast は失敗メッセージを表示し、「Update & restart」を再度押せる状態に戻る (I-U7)
  expect(await $(STATUS).getAttribute('role')).toBe('alert');
  expect(await $(INSTALL).isEnabled()).toBe(true);
  // 「View release」は引き続き使える。Toast も閉じられる
  expect(await $(VIEW_RELEASE).isDisplayed()).toBe(true);
  expect(await $(VIEW_RELEASE).isEnabled()).toBe(true);
  expect(await $(DISMISS).isEnabled()).toBe(true);
  expect(await $(LATEST).getText()).toBe('9.9.8');
  // アプリは再起動しない: 同じ webview / process のまま (Given で置いた目印が残っている)。
  // インストールしたパッケージも起動されていない。何もインストールされていない
  expect(await readSentinel()).toBe(SENTINEL);
  expect(existsSync(MARKER)).toBe(false);
  expect(readFileSync(APPIMAGE)).toEqual(appImageBefore);
}

/** 自動リトライしない: しばらく待っても updater への request が増えない。 */
async function expectNoAutomaticRetry(): Promise<void> {
  const latest = requestCount('/latest.json');
  const download = requestCount('/download');
  await browser.pause(3000);
  expect(requestCount('/latest.json')).toBe(latest);
  expect(requestCount('/download')).toBe(download);
  expect(await readSentinel()).toBe(SENTINEL);
}

describe('scenario:s25-install-update-failure', () => {
  let appImageBefore: Buffer;

  before(() => {
    appImageBefore = readFileSync(APPIMAGE);
  });

  it('given — validation#s25-given: Toast に新バージョンを表示中で、「Update & restart」を押せる', async () => {
    // Given: current < 9.9.8、InstallationKind::Bundled(Deb) → インストール操作を提示する (I-U6)
    const toast = await $(TOAST);
    await toast.waitForExist({ timeout: 15000 });
    expect(await $(LATEST).getText()).toBe('9.9.8');
    await (await $(INSTALL)).waitForDisplayed({ timeout: 5000 });
    expect(await $(INSTALL).isEnabled()).toBe(true);
    expect(await $(STATUS).isExisting()).toBe(false);

    // 以降の「再起動しない」を同一 process の継続として観測するための目印
    await browser.execute((value: string) => {
      (window as unknown as { __s25Sentinel?: string }).__s25Sentinel = value;
    }, SENTINEL);
    expect(await readSentinel()).toBe(SENTINEL);
  });

  it('step 1 — validation#s25-then: download がネットワークエラーで失敗すると失敗を表示し、アプリは継続して再度押せる', async () => {
    // When: ユーザが「Update & restart」を押し、download が失敗する
    setMode('download-fail');
    const downloadsBefore = requestCount('/download');
    await clickInstallAndWaitForFailure();
    expect(requestCount('/download')).toBe(downloadsBefore + 1);

    // Then: 失敗を表示し、アプリは再起動せず、再度押せる状態に戻る
    await expectRecoveredFromFailure(appImageBefore);
    // Then: 自動リトライはしない
    await expectNoAutomaticRetry();

    // Then: UI に返るエラーは InstallUpdateError::DownloadFailed
    const result = await invokeInstall();
    expect(result.ok).toBe(false);
    expect(result.ok ? '' : result.kind).toBe('DownloadFailed');
  });

  it('step 2 — validation#s25-then: install が失敗 (署名検証失敗) しても失敗を表示し、アプリは継続して再度押せる', async () => {
    // When: 失敗表示から再び「Update & restart」を押し、今度は install (署名検証) が失敗する
    setMode('bad-signature');
    const downloadsBefore = requestCount('/download');
    await clickInstallAndWaitForFailure();
    expect(requestCount('/download')).toBe(downloadsBefore + 1);

    // Then: 失敗を表示し、アプリは再起動せず、再度押せる状態に戻る。何もインストールされない
    await expectRecoveredFromFailure(appImageBefore);
    // Then: 自動リトライはしない
    await expectNoAutomaticRetry();

    // Then: UI に返るエラーは InstallUpdateError::InstallFailed
    const result = await invokeInstall();
    expect(result.ok).toBe(false);
    expect(result.ok ? '' : result.kind).toBe('InstallFailed');
  });
});
