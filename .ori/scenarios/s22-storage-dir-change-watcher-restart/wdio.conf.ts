// @ori-generated scenario:s22-storage-dir-change-watcher-restart
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');
const CONFIG_IDENTIFIER = 'com.komenzar.promptnotes';

let tmpDir: string;

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
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s22-');
    const oldDir = join(tmpDir, 'old');
    const newDir = join(tmpDir, 'new');
    const configHome = join(tmpDir, 'config');
    const dataHome = join(tmpDir, 'data');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    mkdirSync(oldDir, { recursive: true });
    mkdirSync(newDir, { recursive: true });
    mkdirSync(appConfigDir, { recursive: true });
    mkdirSync(dataHome, { recursive: true });

    // S22 Given: old dir に Note 3 件 (watcher の初期監視対象)、new dir に別の Note 1 件。
    seedNote(oldDir, '20260601100000', 'old-alpha');
    seedNote(oldDir, '20260615100000', 'old-beta');
    seedNote(oldDir, '20260626100000', 'old-gamma');
    seedNote(newDir, '20260701100000', 'new-delta');

    // S22 Given: Settings (storage_dir = old dir) を app_config_dir/settings.json に seed する。
    const settingsPath = join(appConfigDir, 'settings.json');
    writeFileSync(
      settingsPath,
      JSON.stringify(
        {
          storage_dir: oldDir,
          theme: 'System',
          sort_preference: { field: 'created_at', direction: 'desc' },
        },
        null,
        2,
      ),
      'utf-8',
    );

    // Tauri の app_config_dir()/app_data_dir() は XDG_* を尊重するため、実 user 環境から隔離する。
    // TAURI_TEST_STORAGE_DIR override は使わない (resolve_storage_dir が override を最優先するため
    // storage_dir が固定され、new dir への watcher 切替を検証できない)。
    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = dataHome;
    process.env.TAURI_TEST_S22_OLD_DIR = oldDir;
    process.env.TAURI_TEST_S22_NEW_DIR = newDir;
    process.env.TAURI_TEST_S22_SETTINGS = settingsPath;
    console.log(`[scenario:s22] old=${oldDir} new=${newDir} settings=${settingsPath}`);
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s22] cleaned temp dir: ${tmpDir}`);
    }
  },
};
