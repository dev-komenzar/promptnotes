// @ori-generated scenario:s15-same-second-edits
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
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s15-');
    const notesDir = join(tmpDir, 'notes');
    const configHome = join(tmpDir, 'config');
    const dataHome = join(tmpDir, 'data');
    mkdirSync(notesDir, { recursive: true });
    mkdirSync(join(configHome, 'com.komenzar.promptnotes'), { recursive: true });
    mkdirSync(dataHome, { recursive: true });

    // S15 Given: Note A (updatedAt=12:00:00) と、相対順序検証用の Note B (11:00:00)。
    seedNote(notesDir, '20260620120000', 'hello');
    seedNote(notesDir, '20260620110000', 'beta');

    // 固定秒ファイル: テストがステップ間に RFC3339 を書き込み、auto_save_note の
    // Clock を debug seam 経由で pin する (production release build はこの env を読まない)。
    const fixedNowFile = join(tmpDir, 'fixed-now');
    writeFileSync(fixedNowFile, 'system');

    process.env.XDG_CONFIG_HOME = configHome;
    process.env.XDG_DATA_HOME = dataHome;
    process.env.TAURI_TEST_STORAGE_DIR = notesDir;
    process.env.TAURI_TEST_FIXED_NOW_FILE = fixedNowFile;
    console.log(`[scenario:s15] notes=${notesDir} fixed-now=${fixedNowFile}`);
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s15] cleaned temp dir: ${tmpDir}`);
    }
  },
};
