// @ori-generated scenario:s23-update-same-version-once
import { createServer, type Server } from 'node:http';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
 * S23 の更新確認 endpoint を差し替える local HTTP server。
 * version ファイルの内容 (例: 9.9.8 / 9.9.9) を version に持つ有効な latest.json を request ごとに返す。
 * テストは version ファイルを書き換えて「定期確認で見つかるバージョン」を切り替える。
 */
function startUpdaterServer(versionFile: string): Promise<number> {
  return new Promise((resolvePort, reject) => {
    const server = createServer((_req, res) => {
      const version = readFileSync(versionFile, 'utf-8').trim();
      const body = JSON.stringify({
        version,
        notes: `S23 E2E release notes ${version}`,
        platforms: {
          'linux-x86_64': {
            signature: 'unused-by-check',
            url: 'http://127.0.0.1:1/download',
          },
        },
      });
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(body);
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
    // XDG 隔離は Linux でしか効かない。他 platform では実ユーザ領域を汚すため実行しない
    if (process.platform !== 'linux') {
      throw new SevereServiceError(
        `scenario:s23 は Linux のみ対応 (XDG による storage 隔離が効かない: ${process.platform})`,
      );
    }
    if (await isPortInUse(DRIVER_PORT)) {
      throw new SevereServiceError(
        `tauri-driver port :${DRIVER_PORT} が使用中。残留 tauri-driver の可能性があるため ` +
          '`pkill tauri-driver` で停止してから再実行してください',
      );
    }

    tmpDir = mkdtempSync(join(tmpdir(), 'promptnotes-scenario-s23-'));
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    mkdirSync(notesDir, { recursive: true });
    mkdirSync(appConfigDir, { recursive: true });
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

    // Given: 起動時の確認で 9.9.8 (current より新しい) が見つかる状態にする
    const versionFile = join(tmpDir, 'updater-version');
    writeFileSync(versionFile, '9.9.8');
    const port = await startUpdaterServer(versionFile);

    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = join(tmpDir, 'data');
    process.env.XDG_CACHE_HOME = join(tmpDir, 'cache');
    process.env.XDG_STATE_HOME = join(tmpDir, 'state');
    process.env.ORI_SCENARIO_TMP = tmpDir;
    process.env.TAURI_TEST_S23_UPDATER_VERSION_FILE = versionFile;
    // debug build 限定 seam (check_for_updates/infrastructure.rs)。release build では compile out
    process.env.TAURI_TEST_UPDATER_ENDPOINT = `http://127.0.0.1:${port}/latest.json`;
    console.log(`[scenario:s23] tmp=${tmpDir} updater=http://127.0.0.1:${port}/latest.json`);
  },

  onComplete: async () => {
    await new Promise<void>((resolveClose) => {
      if (!updaterServer) return resolveClose();
      updaterServer.close(() => resolveClose());
    });
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s23] cleaned temp dir: ${tmpDir}`);
    }
  },
};
