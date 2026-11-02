// @ori-generated scenario:s25-install-update-failure
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
// download 応答を遅らせ、install 中の「Updating…」表示を観測できるようにする
const DOWNLOAD_DELAY_MS = 1000;

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
 * updater 用の鍵を実行時に生成し、ファイルに署名する (鍵も署名済みパッケージもコミットしない)。
 * `cargo tauri signer` は nix devShell の cargo-tauri。署名は latest.json に書く形式 (base64)。
 */
function generateKey(keyDir: string): { keyPath: string; pubkey: string } {
  const keyPath = join(keyDir, 'updater.key');
  execFileSync('cargo', ['tauri', 'signer', 'generate', '--ci', '-p', '', '-w', keyPath, '-f'], SIGNER_OPTS);
  return { keyPath, pubkey: readFileSync(`${keyPath}.pub`, 'utf-8').trim() };
}

function sign(keyPath: string, filePath: string): string {
  execFileSync('cargo', ['tauri', 'signer', 'sign', '-f', keyPath, '-p', '', filePath], SIGNER_OPTS);
  return readFileSync(`${filePath}.sig`, 'utf-8').trim();
}

const SIGNER_OPTS = { stdio: 'pipe' as const, env: { ...process.env, CI: 'true' } };

type ServerMode = 'download-fail' | 'bad-signature';

/**
 * S25 の更新配信 server。mode ファイルの内容で失敗の種類を切り替える。
 * - `download-fail`: `/download` が HTTP 500 → updater の download 失敗 (DownloadFailed)
 * - `bad-signature`: `/download` はパッケージを返すが、latest.json の署名は別ファイルのもの
 *   → updater の署名検証失敗 (InstallFailed、I-U7)
 * request 数を path ごとに JSON ファイルへ記録し、テストが「自動リトライしない」を検証する。
 */
function startUpdaterServer(opts: {
  pkgPath: string;
  goodSignature: string;
  badSignature: string;
  modeFile: string;
  countFile: string;
}): Promise<number> {
  const counts: Record<string, number> = {};
  return new Promise((resolvePort, reject) => {
    const server = createServer((req, res) => {
      const path = (req.url ?? '').split('?')[0];
      counts[path] = (counts[path] ?? 0) + 1;
      writeFileSync(opts.countFile, JSON.stringify(counts));
      const mode = readFileSync(opts.modeFile, 'utf-8').trim() as ServerMode;
      const addr = server.address();
      const port = addr && typeof addr === 'object' ? addr.port : 0;
      if (path === '/latest.json') {
        const signature = mode === 'bad-signature' ? opts.badSignature : opts.goodSignature;
        const body = JSON.stringify({
          version: RELEASE_VERSION,
          notes: `S25 E2E release notes ${RELEASE_VERSION}`,
          platforms: {
            'linux-x86_64': { signature, url: `http://127.0.0.1:${port}/download` },
          },
        });
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(body);
        return;
      }
      if (path === '/download') {
        setTimeout(() => {
          if (mode === 'download-fail') {
            res.writeHead(500);
            res.end('S25: simulated download failure');
            return;
          }
          const bytes = readFileSync(opts.pkgPath);
          res.writeHead(200, {
            'content-type': 'application/octet-stream',
            'content-length': String(bytes.length),
          });
          res.end(bytes);
        }, DOWNLOAD_DELAY_MS);
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
        `scenario:s25 は Linux のみ対応 (XDG による storage 隔離 / AppImage install が前提: ${process.platform})`,
      );
    }
    if (await isPortInUse(DRIVER_PORT)) {
      throw new SevereServiceError(
        `tauri-driver port :${DRIVER_PORT} が使用中。残留 tauri-driver の可能性があるため ` +
          '`pkill tauri-driver` で停止してから再実行してください',
      );
    }

    tmpDir = mkdtempSync(join(tmpdir(), 'promptnotes-scenario-s25-'));
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    const installDir = join(tmpDir, 'install');
    const releaseDir = join(tmpDir, 'release');
    for (const d of [notesDir, appConfigDir, installDir, releaseDir]) mkdirSync(d, { recursive: true });
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
    // (updater plugin の Linux install は APPIMAGE が指すファイルを置き換える)。失敗 scenario なので不変のはず
    const appImage = join(installDir, 'PromptNotes.AppImage');
    const appImageContent = '#!/bin/sh\n# S25: 失敗 scenario の偽 AppImage (置き換えられないこと)\nexit 0\n';
    writeFileSync(appImage, appImageContent);
    chmodSync(appImage, 0o755);

    // 新バージョンのテストパッケージ。起動されたら marker を書く (= 再起動したことの検出)
    const marker = join(tmpDir, 'restarted.marker');
    const pkgPath = join(releaseDir, `PromptNotes_${RELEASE_VERSION}_amd64.AppImage`);
    writeFileSync(pkgPath, `#!/bin/sh\necho '${RELEASE_VERSION}' > '${marker}'\n`);
    const decoyPath = join(releaseDir, 'decoy.bin');
    writeFileSync(decoyPath, 'S25: this is not the package that is served\n');
    const { keyPath, pubkey } = generateKey(tmpDir);
    const goodSignature = sign(keyPath, pkgPath);
    const badSignature = sign(keyPath, decoyPath);

    const modeFile = join(tmpDir, 'updater-mode');
    writeFileSync(modeFile, 'download-fail');
    const countFile = join(tmpDir, 'updater-requests.json');
    writeFileSync(countFile, '{}');
    const port = await startUpdaterServer({ pkgPath, goodSignature, badSignature, modeFile, countFile });

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
    // Given: InstallationKind::Bundled(Deb)
    process.env.TAURI_TEST_INSTALLATION_KIND = 'deb';
    // テストが参照するパス
    process.env.TAURI_TEST_S25_APPIMAGE = appImage;
    process.env.TAURI_TEST_S25_MARKER = marker;
    process.env.TAURI_TEST_S25_MODE_FILE = modeFile;
    process.env.TAURI_TEST_S25_REQUEST_COUNTS = countFile;
    console.log(`[scenario:s25] tmp=${tmpDir} updater=http://127.0.0.1:${port}/latest.json`);
  },

  onComplete: async () => {
    await new Promise<void>((resolveClose) => {
      if (!updaterServer) return resolveClose();
      updaterServer.close(() => resolveClose());
    });
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s25] cleaned temp dir: ${tmpDir}`);
    }
  },
};
