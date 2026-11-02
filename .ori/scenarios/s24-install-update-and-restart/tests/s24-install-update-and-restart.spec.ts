// @ori-generated scenario:s24-install-update-and-restart
//
// 検証対象: validation.md#s24-install-update-and-restart
// runner: wdio (tauri) — compose-service 参加者なし
// given: wdio onPrepare が XDG_* を per-run temp に隔離し、storage_dir を temp に向けた settings.json と Note A を seed する。
//        更新配信は 127.0.0.1 の local server (debug seam TAURI_TEST_UPDATER_ENDPOINT)。テストパッケージは
//        onPrepare が実行時に生成した鍵で署名し、debug seam TAURI_TEST_UPDATER_PUBKEY でその公開鍵を検証に使わせる。
//        配布形式は debug seam TAURI_TEST_INSTALLATION_KIND=appimage、APPIMAGE は temp の偽 AppImage を指す。
//
// 核: 「Update & restart」で最新リリースを取得し直し、download + 署名検証 + install し、
//     編集中の Note A の永続化を済ませてから、インストールしたバージョンで再起動する (I-U5〜I-U8)。
//
// 観測方法 (spec.md#impl-notes):
//   - install: 偽 AppImage の中身が配信したテストパッケージと一致する (updater plugin の AppImage install)
//   - 再起動: AppHandle::restart は APPIMAGE (= インストール済みパッケージ) を起動する。パッケージは起動時に
//     marker と「その時点の Note A ファイル」の snapshot を書く → 再起動前に Note A が永続化済みだったことを示す
//   - 「Update & restart」のクリックで editor は blur し (blur flush)、AutoSave debounce (500ms) も install 中に成立するため、
//     押す前の編集だけでは「再起動前の Flush」を外しても永続化されてしまう。そこで download を server 側で保留し、
//     install 中に Note A へ追加の編集をして未保存であることを確認した直後に download を解放する。
//     この編集は debounce 成立前に install 完了 → page-main の Flush → 再起動 の経路でしか永続化されない
//   - 再起動でアプリ process は終了し WebDriver session は失われるため、再起動後の検証は fs のみで行う

import { existsSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const NOTES_DIR = process.env.TAURI_TEST_S24_NOTES_DIR!;
const APPIMAGE = process.env.TAURI_TEST_S24_APPIMAGE!;
const PACKAGE = process.env.TAURI_TEST_S24_PACKAGE!;
const MARKER = process.env.TAURI_TEST_S24_MARKER!;
const SNAPSHOT = process.env.TAURI_TEST_S24_SNAPSHOT!;
const REQUEST_COUNTS = process.env.TAURI_TEST_S24_REQUEST_COUNTS!;
const RELEASE_DOWNLOAD_URL = process.env.TAURI_TEST_S24_RELEASE_DOWNLOAD_URL!;

const NOTE_A = '20260620120000';
// 同じ文字を連続させない: WebKitWebDriver の key action は連続する同一キーの 2 打目を落とすことがある
const EDIT_SUFFIX = ' S24-pending-edit';
const EDIT_DURING_INSTALL = ' S24-typed-while-updating';
// 最後の 1 打鍵。この打鍵の AutoSave debounce が成立する前に再起動したことを時刻で確認する
const LAST_KEY = '!';
// domain の AutoSave debounce (README / S2: キー入力後 500ms)
const AUTOSAVE_DEBOUNCE_MS = 500;

const TOAST = '[data-testid="widget.widget-update-toast.root"]';
const LATEST = '[data-testid="widget.widget-update-toast.latest-version"]';
const INSTALL = '[data-testid="widget.widget-update-toast.install-update"]';
const STATUS = '[data-testid="widget.widget-update-toast.install-status"]';
const DISMISS = '[data-testid="widget.widget-update-toast.dismiss"]';

function requestCount(path: string): number {
  const counts = JSON.parse(readFileSync(REQUEST_COUNTS, 'utf-8')) as Record<string, number>;
  return counts[path] ?? 0;
}

/** Feed の Block (testid 契約 `page.page-main.block`、動的要素は data-key = note id)。 */
function block(id: string) {
  return $(`[data-testid="page.page-main.block"][data-key="${id}"]`);
}

function readNoteA(): string {
  return readFileSync(join(NOTES_DIR, `${NOTE_A}.md`), 'utf-8');
}

/**
 * Note A を EDITING にして末尾に text を入力する (AutoSave debounce timer が走る)。
 * 戻り値はキー送信直前の時刻 (= 入力した打鍵の時刻の下限)。
 */
async function typeIntoNoteA(text: string): Promise<number> {
  const el = await block(NOTE_A);
  await el.waitForExist({ timeout: 15000 });
  if ((await el.getAttribute('data-block-state')) !== 'EDITING') {
    await el.click();
    await browser.waitUntil(async () => (await el.getAttribute('data-block-state')) === 'EDITING', {
      timeout: 5000,
      timeoutMsg: 'Note A did not enter EDITING',
    });
  }
  const editor = await el.$('.cm-editor .cm-content');
  await editor.waitForExist({ timeout: 3000 });
  await editor.click();
  await browser.keys(['Control', 'End']);
  const sentAt = Date.now();
  await browser.keys(text.split(''));
  // keys の resolve は editor への反映を待たないため、入力し終えたことを DOM で確認する
  let last = '';
  await browser
    .waitUntil(async () => (last = await editor.getText()).includes(text), { timeout: 3000 })
    .catch(() => {
      throw new Error(`Note A editor did not receive ${JSON.stringify(text)}: ${JSON.stringify(last)}`);
    });
  return sentAt;
}

async function waitForFile(path: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!existsSync(path)) {
    if (Date.now() > deadline) throw new Error(`file was not created within ${timeoutMs}ms: ${path}`);
    await new Promise((r) => setTimeout(r, 100));
  }
}

describe('scenario:s24-install-update-and-restart', () => {
  let latestJsonBeforeInstall = 0;

  it('given — validation#s24-given: 起動時の確認で新バージョンの Toast を表示し、Note A を EDITING にする', async () => {
    // Given: 起動時の確認で 9.9.8 を検出し Toast 表示中。配布形式は AppImage なので「Update & restart」が出る (I-U6)
    const toast = await $(TOAST);
    await toast.waitForExist({ timeout: 15000 });
    expect(await $(LATEST).getText()).toBe('9.9.8');
    await (await $(INSTALL)).waitForDisplayed({ timeout: 5000 });
    expect(await $(INSTALL).isEnabled()).toBe(true);

    // Given: Note A が EDITING 状態 (AutoSave debounce timer 中の未保存の編集がある)
    await typeIntoNoteA(EDIT_SUFFIX);
    expect(readNoteA()).not.toContain(EDIT_SUFFIX);

    latestJsonBeforeInstall = requestCount('/latest.json');
    expect(latestJsonBeforeInstall).toBeGreaterThanOrEqual(1);
    expect(requestCount('/download')).toBe(0);
  });

  it('step 1 — validation#s24-then: 「Update & restart」を押すと処理中は「Updating…」を表示し、ボタンは押せない', async () => {
    // When: ユーザが Toast の「Update & restart」を押す
    await $(INSTALL).click();

    // Then: 処理中は Toast が「Updating…」を表示し、ボタン (と dismiss) は押せない
    //   (download は server 側で保留しているので、この間に観測できる)
    const status = await $(STATUS);
    await status.waitForExist({ timeout: 2000 });
    expect(await status.getText()).toBe('Updating…');
    expect(await $(INSTALL).isEnabled()).toBe(false);
    expect(await $(DISMISS).isEnabled()).toBe(false);
    // download 要求は来ている (保留中)
    await browser.waitUntil(async () => requestCount('/download') === 1, {
      timeout: 5000,
      timeoutMsg: 'updater did not request the package download',
    });
  });

  it('step 2 — validation#s24-then: 最新リリースを取得し直し、download + 署名検証 + install して、Note A を永続化してから再起動する', async () => {
    // install 中にも Note A の編集を続ける。debounce 成立前 (未保存) のうちに download を解放する
    await typeIntoNoteA(EDIT_DURING_INSTALL);
    const beforeLastKey = await typeIntoNoteA(LAST_KEY);
    expect(readNoteA()).not.toContain(`${EDIT_DURING_INSTALL}${LAST_KEY}`);
    await fetch(RELEASE_DOWNLOAD_URL);

    // Then: インストール後の再起動で、インストールしたパッケージ (新バージョン) が起動される
    //   ここから先はアプリ process が終了しているため、WebDriver は使わず fs だけで検証する
    await waitForFile(MARKER, 30000);
    await waitForFile(SNAPSHOT, 5000);
    expect(readFileSync(MARKER, 'utf-8').trim()).toBe('9.9.8');

    // Then: UpdateChannel::install() は最新リリースを取得し直している (通知時の情報を使い回さない)
    expect(requestCount('/latest.json')).toBeGreaterThan(latestJsonBeforeInstall);
    // Then: updater plugin が download を 1 回行い、署名検証を通ったパッケージを install した
    expect(requestCount('/download')).toBe(1);
    expect(readFileSync(APPIMAGE)).toEqual(readFileSync(PACKAGE));

    // Then: 再起動の時点で Note A の編集 (install 完了直前の未保存分を含む) は永続化済み
    //   (全 Note を Flush し、永続化を待ってから再起動、I-U8)
    const atRestart = readFileSync(SNAPSHOT, 'utf-8');
    expect(atRestart).toContain(`alpha body${EDIT_SUFFIX}${EDIT_DURING_INSTALL}${LAST_KEY}`);
    // その永続化は AutoSave debounce ではなく再起動前の Flush による: Note A の最後の書き込みは、最後の打鍵の
    // debounce が成立するより前に起きている (成立後の debounce は同一 body のため書き込まない、S9)
    const persistedAfterLastKeyMs = statSync(join(NOTES_DIR, `${NOTE_A}.md`)).mtimeMs - beforeLastKey;
    console.log(`[scenario:s24] Note A persisted ${Math.round(persistedAfterLastKeyMs)}ms after the last keystroke`);
    expect(persistedAfterLastKeyMs).toBeLessThan(AUTOSAVE_DEBOUNCE_MS);
  });
});
