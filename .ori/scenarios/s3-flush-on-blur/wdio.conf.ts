// @ori-generated scenario:s3-flush-on-blur — wdio.conf.ts
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// runtime.binary (build-then-test) を絶対パスで解決。
// .ori/architecture.md workspace.apps[].runtime.binary 準拠。
const BINARY = resolve(
  __dirname,
  '../../../apps/promptnotes/src-tauri/target/debug/promptnotes'
);

export const config = {
  runner: 'local' as const,
  specs: ['./tests/**/*.spec.ts'],
  maxInstances: 1,
  services: [['@wdio/tauri-service', { driverProvider: 'external' }]],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': { application: BINARY }
    }
  ],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 60_000 },
  reporters: ['spec'],
  // compose-service 系参加者なし（全参加者は local tauri app のみ）のため
  // onPrepare / onComplete は compose lifecycle を実行しない
};