// @ori-generated scenario:s5-delete-undo-in-window
//
// WDIO runner config — tauri (local mode) app を external WebDriver で制御。
// compose-service 参加者ゼロのため docker compose の起動/停止はスキップ。

import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
// .ori/architecture.md → app promptnotes: mode=local, binary (relative from project root)
const PROJECT_ROOT = resolve(__dirname, '../../..');
const BINARY = resolve(PROJECT_ROOT, 'apps/promptnotes/src-tauri/target/debug/promptnotes');

export const config = {
  runner: 'local',
  specs: ['./tests/**/*.spec.ts'],

  maxInstances: 1,
  logLevel: 'warn',

  services: [['@wdio/tauri-service', { driverProvider: 'external' }]],

  capabilities: [{
    browserName: 'tauri',
    'tauri:options': {
      application: BINARY,
      // テスト用にストレージディレクトリを分離
      env: {
        STORAGE_DIR: resolve(__dirname, '__test_storage__'),
      },
    },
  }],

  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 30000,
  },

  reporters: ['spec'],

  beforeSession: async () => {
    // WDIO のテストファイルは node fs にアクセスできない（tauri context で
    // 実行される）ため、file-level fixture は spec 内の before/beforeEach
    // で実施する。
    //
    // NOTE: beforeEach/afterEach 内の fs 操作（node:fs）は require
    // が解決されないため使用不可。spec 内でインポートする。
  },
};