// @ori-generated scenario:s14-update-check-failure
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { createServer, type Server } from 'node:http';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');

let tmpDir: string;
let updaterServer: Server | undefined;

function seedNote(dir: string, id: string, body: string): void {
  const content = [
    '---',
    `createdAt: ${id}`,
    `updatedAt: ${id}`,
    'tags: []',
    '---',
    body,
  ].join('\n');
  writeFileSync(join(dir, `${id}.md`), content, 'utf-8');
}

/**
 * S14 の更新チェック endpoint を差し替えるための local HTTP server。
 * mode ファイル (`fail` / `release`) を request ごとに読み、応答を切り替える:
 *   - fail:    socket を切断してネットワーク断を再現 → tauri updater は network error
 *   - release: 有効な latest.json (current より新しい version) を返す → NewVersionDetected 発行
 */
function startUpdaterServer(modeFile: string): Promise<number> {
  return new Promise((resolvePort, reject) => {
    const server = createServer((req, res) => {
      const mode = readFileSync(modeFile, 'utf-8').trim();
      if (mode === 'release') {
        const body = JSON.stringify({
          version: '9.9.9',
          notes: 'S14 E2E release notes',
          platforms: {
            'linux-x86_64': {
              signature: 'unused-by-check',
              url: 'http://127.0.0.1:1/download',
            },
          },
        });
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(body);
        return;
      }
      res.socket?.destroy();
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

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./tests/**/*.spec.ts'],
  maxInstances: 1,
  services: [['@wdio/tauri-service', { driverProvider: 'external' }]],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': { application: BINARY },
    },
  ],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 120_000 },
  reporters: ['spec'],
  logLevel: 'warn',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  onPrepare: async () => {
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s14-');
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const dataHome = join(tmpDir, 'data');
    mkdirSync(notesDir, { recursive: true });
    mkdirSync(join(configHome, 'com.komenzar.promptnotes'), { recursive: true });
    mkdirSync(dataHome, { recursive: true });

    // S14 Given: 失敗後もユーザの作業が妨げられないことを示すため Note を 1 件 seed する。
    seedNote(notesDir, '20260620120000', 's14 seed body');

    const modeFile = join(tmpDir, 'updater-mode');
    writeFileSync(modeFile, 'fail');
    const port = await startUpdaterServer(modeFile);

    // Tauri の app_config_dir()/app_data_dir() と note コマンドの storage_dir を
    // 実 user 環境から隔離する。更新チェック endpoint は debug seam
    // (TAURI_TEST_UPDATER_ENDPOINT) 経由で local server に向ける。
    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = dataHome;
    process.env.TAURI_TEST_STORAGE_DIR = notesDir;
    process.env.TAURI_TEST_S14_NOTES_DIR = notesDir;
    process.env.TAURI_TEST_S14_UPDATER_MODE_FILE = modeFile;
    process.env.TAURI_TEST_UPDATER_ENDPOINT = `http://127.0.0.1:${port}/latest.json`;
    console.log(`[scenario:s14] notes=${notesDir} updater=http://127.0.0.1:${port}/latest.json`);
  },

  onComplete: async () => {
    await new Promise<void>((resolveClose) => {
      if (!updaterServer) return resolveClose();
      updaterServer.close(() => resolveClose());
    });
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s14] cleaned temp dir: ${tmpDir}`);
    }
  },
};
