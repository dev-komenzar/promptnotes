// @ori-generated scenario:s5-delete-undo-in-window
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');

let tmpDir: string;

function seedNote(id: string, body: string): void {
  const content = [
    '---',
    `createdAt: ${id}`,
    `updatedAt: ${id}`,
    'tags: []',
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
  mochaOpts: { ui: 'bdd', timeout: 30_000 },
  reporters: ['spec'],

  onPrepare: async () => {
    tmpDir = mkdtempSync(tmpdir() + '/promptnotes-scenario-s5-');
    seedNote('20260620120000', 'hello');
    process.env.TAURI_TEST_STORAGE_DIR = tmpDir;
  },

  onComplete: async () => {
    if (tmpDir) {
      rmSync(tmpDir, { recursive: true, force: true });
    }
  },
};