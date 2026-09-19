// @ori-generated scenario:s11-storage-dir-change
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
  capabilities: [{
    browserName: 'tauri',
    'tauri:options': { application: BINARY },
  }],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 120_000 },
  reporters: ['spec'],
  logLevel: 'warn',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  onPrepare: async () => {
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s11-');
    const oldDir = join(tmpDir, 'old');
    const newDir = join(tmpDir, 'new');
    const configHome = join(tmpDir, 'config');
    const dataHome = join(tmpDir, 'data');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    mkdirSync(oldDir, { recursive: true });
    mkdirSync(newDir, { recursive: true });
    mkdirSync(appConfigDir, { recursive: true });
    mkdirSync(dataHome, { recursive: true });

    // S11 Given: old dir に Note 3 件、new dir に別の Note 1 件
    seedNote(oldDir, '20260601100000', 'alpha');
    seedNote(oldDir, '20260615100000', 'beta');
    seedNote(oldDir, '20260626100000', 'gamma');
    seedNote(newDir, '20260701100000', 'delta-from-new-dir');

    // S11 Given: Settings (storage_dir = old dir) を app_config_dir/settings.json に seed する
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
    // TAURI_TEST_STORAGE_DIR override は使わない (storage_dir が固定され再起動後の new dir を検証できない)。
    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = dataHome;
    process.env.TAURI_TEST_S11_OLD_DIR = oldDir;
    process.env.TAURI_TEST_S11_NEW_DIR = newDir;
    process.env.TAURI_TEST_S11_SETTINGS = settingsPath;
    console.log(`[scenario:s11] old=${oldDir} new=${newDir} settings=${settingsPath}`);
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s11] cleaned temp dir: ${tmpDir}`);
    }
  },
};
