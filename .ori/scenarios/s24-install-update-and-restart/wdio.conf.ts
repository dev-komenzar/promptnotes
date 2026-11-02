// @ori-generated scenario:s24-install-update-and-restart
import { execFileSync } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
// onPrepare で普通の Error を throw しても wdio launcher はログに出すだけで続行する。停止には SevereServiceError
import { SevereServiceError } from 'webdriverio';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');
const CONFIG_IDENTIFIER = 'com.komenzar.promptnotes';
// tauri-driver の待受 port（@wdio/tauri-service の既定値）
const DRIVER_PORT = 4444;
// 配信する「新バージョン」。test build の current (ビルド時 version) より確実に新しい
const RELEASE_VERSION = '9.9.8';
const NOTE_A = '20260620120000';

let tmpDir = '';
let updaterServer: Server | undefined;

const isPortInUse = (port: number) =>
  new Promise<boolean>((done) => {
    const socket = createConnection({ port, host: '127.0.0.1' });
    socket.setTimeout(1000, () => {
      socket.destroy();
      done(false);
    });
    socket.once('connect', () => {
      socket.destroy();
      done(true);
    });
    socket.once('error', () => done(false));
  });

function seedNote(dir: string, id: string, body: string): void {
  const content = ['---', `createdAt: ${id}`, `updatedAt: ${id}`, 'tags: []', '---', body].join('\n');
  writeFileSync(join(dir, `${id}.md`), content, 'utf-8');
}

/**
 * updater 用の鍵を実行時に生成し、テストパッケージに署名する (鍵も署名済みパッケージもコミットしない)。
 * `cargo tauri signer` は nix devShell の cargo-tauri。戻り値は (公開鍵, 署名) で、どちらも
 * latest.json / tauri.conf.json に書く形式 (base64)。
 */
function signPackage(keyDir: string, pkgPath: string): { pubkey: string; signature: string } {
  const keyPath = join(keyDir, 'updater.key');
  const opts = { stdio: 'pipe' as const, env: { ...process.env, CI: 'true' } };
  execFileSync('cargo', ['tauri', 'signer', 'generate', '--ci', '-p', '', '-w', keyPath, '-f'], opts);
  execFileSync('cargo', ['tauri', 'signer', 'sign', '-f', keyPath, '-p', '', pkgPath], opts);
  return {
    pubkey: readFileSync(`${keyPath}.pub`, 'utf-8').trim(),
    signature: readFileSync(`${pkgPath}.sig`, 'utf-8').trim(),
  };
}

/**
 * S24 の更新配信 server。
 * - `/latest.json`: RELEASE_VERSION を version に持つ有効な latest.json (署名は実署名)
 * - `/download`: 署名済みテストパッケージ。テストが `/release-download` を叩くまで応答を保留する
 *   (install 中の「Updating…」を観測し、install 完了直前に未保存の編集を作るため)
 * request 数を path ごとに JSON ファイルへ記録し、テストが「取得し直し」「download 1 回」を検証する。
 */
function startUpdaterServer(pkgPath: string, signature: string, countFile: string): Promise<number> {
  const counts: Record<string, number> = {};
  let downloadGateOpen = false;
  const heldDownloads: Array<() => void> = [];
  return new Promise((resolvePort, reject) => {
    const server = createServer((req, res) => {
      const path = (req.url ?? '').split('?')[0];
      counts[path] = (counts[path] ?? 0) + 1;
      writeFileSync(countFile, JSON.stringify(counts));
      const addr = server.address();
      const port = addr && typeof addr === 'object' ? addr.port : 0;
      if (path === '/latest.json') {
        const body = JSON.stringify({
          version: RELEASE_VERSION,
          notes: `S24 E2E release notes ${RELEASE_VERSION}`,
          platforms: {
            'linux-x86_64': { signature, url: `http://127.0.0.1:${port}/download` },
          },
        });
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(body);
        return;
      }
      if (path === '/download') {
        const send = () => {
          const bytes = readFileSync(pkgPath);
          res.writeHead(200, {
            'content-type': 'application/octet-stream',
            'content-length': String(bytes.length),
          });
          res.end(bytes);
        };
        if (downloadGateOpen) send();
        else heldDownloads.push(send);
        return;
      }
      if (path === '/release-download') {
        downloadGateOpen = true;
        for (const send of heldDownloads.splice(0)) send();
        res.writeHead(204);
        res.end();
        return;
      }
      res.writeHead(404);
      res.end();
    });
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      if (addr && typeof addr === 'object') {
        updaterServer = server;
        resolvePort(addr.port);
      } else {
        reject(new Error('failed to determine updater server port'));
      }
    });
  });
}

// 'tauri:options' は WebdriverIO.Capabilities の型に無いため、型を明示した定数に切り出す
const tauriCapability: WebdriverIO.Capabilities & { 'tauri:options': { application: string } } = {
  browserName: 'tauri',
  'tauri:options': { application: BINARY },
};

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./tests/**/*.spec.ts'],
  // 1 固定: >1 では tauri-service が XDG_DATA_HOME を tauri-worker-<cid> に上書きし隔離が崩れる
  maxInstances: 1,
  services: [['@wdio/tauri-service', { driverProvider: 'external' }]],
  capabilities: [tauriCapability],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 120_000 },
  reporters: ['spec'],
  logLevel: 'warn',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  onPrepare: async () => {
    // XDG 隔離と AppImage の差し替えは Linux でしか効かない。他 platform では実ユーザ領域を汚すため実行しない
    if (process.platform !== 'linux') {
      throw new SevereServiceError(
        `scenario:s24 は Linux のみ対応 (XDG による storage 隔離 / AppImage install が前提: ${process.platform})`,
      );
    }
    if (await isPortInUse(DRIVER_PORT)) {
      throw new SevereServiceError(
        `tauri-driver port :${DRIVER_PORT} が使用中。残留 tauri-driver の可能性があるため ` +
          '`pkill tauri-driver` で停止してから再実行してください',
      );
    }

    tmpDir = mkdtempSync(join(tmpdir(), 'promptnotes-scenario-s24-'));
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    const installDir = join(tmpDir, 'install');
    const releaseDir = join(tmpDir, 'release');
    for (const d of [notesDir, appConfigDir, installDir, releaseDir]) mkdirSync(d, { recursive: true });
    for (const d of ['data', 'cache', 'state']) mkdirSync(join(tmpDir, d), { recursive: true });

    writeFileSync(
      join(appConfigDir, 'settings.json'),
      JSON.stringify(
        {
          storage_dir: notesDir,
          theme: 'System',
          sort_preference: { field: 'created_at', direction: 'desc' },
        },
        null,
        2,
      ),
      'utf-8',
    );
    // Given: 編集対象の Note A
    seedNote(notesDir, NOTE_A, 'alpha body');

    // 実行中アプリの「AppImage」。updater plugin は APPIMAGE が指すファイルを新パッケージで置き換え、
    // AppHandle::restart は APPIMAGE を起動する (いずれも plugin / tauri の既存挙動)。
    // target/debug/app を上書きさせないため、temp の偽 AppImage に向ける。
    const appImage = join(installDir, 'PromptNotes.AppImage');
    writeFileSync(appImage, '#!/bin/sh\n# S24: install 前の偽 AppImage (起動されないこと)\nexit 0\n');
    chmodSync(appImage, 0o755);

    // 新バージョンのテストパッケージ。再起動で起動されたら、その時点の Note A の永続化内容と
    // 起動されたことを記録する (= 再起動前に Flush が完了していたかを外から観測する)。
    const marker = join(tmpDir, 'restarted.marker');
    const snapshot = join(tmpDir, 'note-a-at-restart.md');
    const pkgPath = join(releaseDir, `PromptNotes_${RELEASE_VERSION}_amd64.AppImage`);
    writeFileSync(
      pkgPath,
      [
        '#!/bin/sh',
        `# S24 E2E: PromptNotes ${RELEASE_VERSION} (test package)`,
        `cat '${join(notesDir, `${NOTE_A}.md`)}' > '${snapshot}'`,
        `echo '${RELEASE_VERSION}' > '${marker}'`,
        '',
      ].join('\n'),
    );
    const { pubkey, signature } = signPackage(tmpDir, pkgPath);

    const countFile = join(tmpDir, 'updater-requests.json');
    writeFileSync(countFile, '{}');
    const port = await startUpdaterServer(pkgPath, signature, countFile);

    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = join(tmpDir, 'data');
    process.env.XDG_CACHE_HOME = join(tmpDir, 'cache');
    process.env.XDG_STATE_HOME = join(tmpDir, 'state');
    process.env.ORI_SCENARIO_TMP = tmpDir;
    process.env.APPIMAGE = appImage;
    // debug build 限定 seam (update_distribution/shared/updater.rs, install_update/infrastructure.rs)。
    // release build では compile out
    process.env.TAURI_TEST_UPDATER_ENDPOINT = `http://127.0.0.1:${port}/latest.json`;
    process.env.TAURI_TEST_UPDATER_PUBKEY = pubkey;
    process.env.TAURI_TEST_INSTALLATION_KIND = 'appimage';
    // テストが参照するパス
    process.env.TAURI_TEST_S24_NOTES_DIR = notesDir;
    process.env.TAURI_TEST_S24_APPIMAGE = appImage;
    process.env.TAURI_TEST_S24_PACKAGE = pkgPath;
    process.env.TAURI_TEST_S24_MARKER = marker;
    process.env.TAURI_TEST_S24_SNAPSHOT = snapshot;
    process.env.TAURI_TEST_S24_REQUEST_COUNTS = countFile;
    process.env.TAURI_TEST_S24_RELEASE_DOWNLOAD_URL = `http://127.0.0.1:${port}/release-download`;
    console.log(`[scenario:s24] tmp=${tmpDir} updater=http://127.0.0.1:${port}/latest.json`);
  },

  onComplete: async () => {
    await new Promise<void>((resolveClose) => {
      if (!updaterServer) return resolveClose();
      updaterServer.close(() => resolveClose());
    });
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s24] cleaned temp dir: ${tmpDir}`);
    }
  },
};
