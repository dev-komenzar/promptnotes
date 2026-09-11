// @ori-generated scenario:s1-note-created-happy
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
// runtime.binary (build-then-test) — Cargo project name = "app"
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/app');

export const config: WebdriverIO.Config = {
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
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  reporters: ['spec'],
  // No onPrepare/onComplete — all participants are local (Tauri desktop).
  // docker-compose not generated (compose-service participants = 0).
};