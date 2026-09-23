// @ori-generated scenario:s13-quit-flush
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');

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
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s13-');
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const dataHome = join(tmpDir, 'data');
    mkdirSync(notesDir, { recursive: true });
    mkdirSync(join(configHome, 'com.komenzar.promptnotes'), { recursive: true });
    mkdirSync(dataHome, { recursive: true });

    // S13 Given: pending flush の対象となる Note A, B, C（連続 Flush の順序検証）と
    // UI pending 検証用の Note D を seed する。各 body は一意で、更新を判別できる。
    seedNote(notesDir, '20260620120000', 'alpha body');
    seedNote(notesDir, '20260620130000', 'beta body');
    seedNote(notesDir, '20260620140000', 'gamma body');
    seedNote(notesDir, '20260620150000', 'delta body');

    // Tauri の app_config_dir()/app_data_dir() と note コマンドの storage_dir を
    // 実 user 環境から隔離する。`resolve_storage_dir` / `list_notes` は
    // TAURI_TEST_STORAGE_DIR を最優先で採用する (note_capture/shared/storage.rs)。
    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = dataHome;
    process.env.TAURI_TEST_STORAGE_DIR = notesDir;
    process.env.TAURI_TEST_S13_NOTES_DIR = notesDir;
    console.log(`[scenario:s13] notes=${notesDir}`);
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s13] cleaned temp dir: ${tmpDir}`);
    }
  },
};
