// @ori-generated scenario:s12-startup-state
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');
const CONFIG_IDENTIFIER = 'com.komenzar.promptnotes';

let tmpDir: string;

function seedNote(
  dir: string,
  id: string,
  createdAt: string,
  updatedAt: string,
  body: string
): void {
  const content = [
    '---',
    `createdAt: ${createdAt}`,
    `updatedAt: ${updatedAt}`,
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
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s12-');
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const dataHome = join(tmpDir, 'data');
    const appConfigDir = join(configHome, CONFIG_IDENTIFIER);
    mkdirSync(notesDir, { recursive: true });
    mkdirSync(appConfigDir, { recursive: true });
    mkdirSync(dataHome, { recursive: true });

    // S12 Given: updatedAt 昇順 = [beta, gamma, alpha]。createdAt 昇順 = [alpha, beta, gamma]、
    // createdAt 降順 (I-S3 default = { created_at, desc }) = [gamma, beta, alpha] と相異なる。
    // これにより「Settings から sort を復元したか / default のままか」を表示順で判別できる。
    seedNote(notesDir, '20260101100000', '20260101100000', '20260601100000', 'alpha');
    seedNote(notesDir, '20260201100000', '20260201100000', '20260401100000', 'beta');
    seedNote(notesDir, '20260301100000', '20260301100000', '20260501100000', 'gamma');

    // S12 Given: Settings 永続化済み (sort_preference = { updated_at, asc })。
    // filter は Settings に含まれない (挥発、Q3 / I-F6)。
    const settingsPath = join(appConfigDir, 'settings.json');
    writeFileSync(
      settingsPath,
      JSON.stringify(
        {
          storage_dir: notesDir,
          theme: 'System',
          sort_preference: { field: 'updated_at', direction: 'asc' },
        },
        null,
        2,
      ),
      'utf-8',
    );

    // Tauri の app_config_dir()/app_data_dir() は XDG_* を尊重するため、実 user 環境から隔離する。
    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = dataHome;
    process.env.TAURI_TEST_S12_NOTES_DIR = notesDir;
    process.env.TAURI_TEST_S12_SETTINGS = settingsPath;
    console.log(`[scenario:s12] notes=${notesDir} settings=${settingsPath}`);
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s12] cleaned temp dir: ${tmpDir}`);
    }
  },
};
