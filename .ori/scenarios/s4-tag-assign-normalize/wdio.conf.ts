// @ori-generated scenario:s4-tag-assign-normalize
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');

let tmpDir: string;

function seedNote(id: string, body: string, tags: string[]): void {
  const content = [
    '---',
    `createdAt: ${id}`,
    `updatedAt: ${id}`,
    `tags: [${tags.join(', ')}]`,
    '---',
    body,
  ].join('\n');
  writeFileSync(join(tmpDir, `${id}.md`), content, 'utf-8');
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
  mochaOpts: { ui: 'bdd', timeout: 60_000 },
  reporters: ['spec'],
  logLevel: 'warn',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  onPrepare: async () => {
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s4-');
    seedNote('20260620120000', 'hello', ['gpt']);
    process.env.TAURI_TEST_STORAGE_DIR = tmpDir;
    console.log(`[scenario:s4] temp storage dir: ${tmpDir}`);
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
      console.log(`[scenario:s4] cleaned temp dir: ${tmpDir}`);
    }
  },
};