// @ori-generated scenario:s26-unbundled-notify-only
import { createServer, type Server } from 'node:http';
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
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

/**
 * S26 の更新確認 server。RELEASE_VERSION を version に持つ有効な latest.json を返す
 * (download 先も用意するが、Unbundled では一度も取りに来ないことを検証する)。
 * request 数を path ごとに JSON ファイルへ記録し、テストが「ネットワークに触れない」を検証する。
 */
function startUpdaterServer(countFile: string): Promise<number> {
  const counts: Record<string, number> = {};
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
          notes: `S26 E2E release notes ${RELEASE_VERSION}`,
          platforms: {
            'linux-x86_64': {
              signature: 'unused-by-unbundled',
              url: `http://127.0.0.1:${port}/download`,
            },
          },
        });
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(body);
        return;
      }
      if (path === '/download') {
        res.writeHead(500);
        res.end('S26: download must not be requested');
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
        `scenario:s26 は Linux のみ対応 (XDG による storage 隔離が前提: ${process.platform})`,
      );
    }
    if (await isPortInUse(DRIVER_PORT)) {
      throw new SevereServiceError(
        `tauri-driver port :${DRIVER_PORT} が使用中。残留 tauri-driver の可能性があるため ` +
          '`pkill tauri-driver` で停止してから再実行してください',
      );
    }

    tmpDir = mkdtempSync(join(tmpdir(), 'promptnotes-scenario-s26-'));
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    const installDir = join(tmpDir, 'install');
    for (const d of [notesDir, appConfigDir, installDir]) mkdirSync(d, { recursive: true });
    for (const d of ['data', 'cache', 'state']) mkdirSync(join(tmpDir, d), { recursive: true });

    // storage_dir を temp dir に向ける (Note の seed は不要)
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

    // 安全策: 万一 install まで進んでも target/debug/app ではなく temp の偽 AppImage が置き換わるようにする
    // (updater plugin の Linux install は APPIMAGE が指すファイルを置き換える)。Unbundled なので不変のはず
    const appImage = join(installDir, 'PromptNotes.AppImage');
    writeFileSync(appImage, '#!/bin/sh\n# S26: Unbundled scenario の偽 AppImage (置き換えられないこと)\nexit 0\n');
    chmodSync(appImage, 0o755);

    const countFile = join(tmpDir, 'updater-requests.json');
    writeFileSync(countFile, '{}');
    const port = await startUpdaterServer(countFile);

    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = join(tmpDir, 'data');
    process.env.XDG_CACHE_HOME = join(tmpDir, 'cache');
    process.env.XDG_STATE_HOME = join(tmpDir, 'state');
    process.env.ORI_SCENARIO_TMP = tmpDir;
    process.env.APPIMAGE = appImage;
    // debug build 限定 seam (update_distribution/shared/updater.rs)。release build では compile out
    process.env.TAURI_TEST_UPDATER_ENDPOINT = `http://127.0.0.1:${port}/latest.json`;
    // Given: InstallationKind::Unbundled。`--no-bundle` の test build の配布形式そのもの (debug build の既定) を使い、
    // 配布形式の seam (TAURI_TEST_INSTALLATION_KIND) は使わない
    delete process.env.TAURI_TEST_INSTALLATION_KIND;
    // テストが参照するパス
    process.env.TAURI_TEST_S26_APPIMAGE = appImage;
    process.env.TAURI_TEST_S26_REQUEST_COUNTS = countFile;
    console.log(`[scenario:s26] tmp=${tmpDir} updater=http://127.0.0.1:${port}/latest.json`);
  },

  onComplete: async () => {
    await new Promise<void>((resolveClose) => {
      if (!updaterServer) return resolveClose();
      updaterServer.close(() => resolveClose());
    });
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s26] cleaned temp dir: ${tmpDir}`);
    }
  },
};
