// @ori-generated scenario:s4-tag-assign-normalize
//
// Runner: wdio (Tauri local mode)
// Binary: build-then-test — runtime.binary を絶対パスで解決
//
// compose-service 系参加者: 0 → docker compose の onPrepare/onComplete は不要

import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');

export const config = {
  runner: 'local',
  specs: ['./tests/**/*.spec.ts'],
  maxInstances: 1,

  // external = tauri-driver (intermediary) + WebKitWebDriver (native)
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
};