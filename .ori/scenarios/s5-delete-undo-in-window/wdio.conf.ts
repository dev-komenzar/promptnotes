// @ori-generated scenario:s5-delete-undo-in-window
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BINARY = resolve(__dirname, '../../../apps/promptnotes/src-tauri/target/debug/promptnotes');

let tmpDir: string;

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

  onPrepare: async ()一流 Having the const mkdtemp resolution the for the I this is a the Let let that for to to for this Let.

OK let me rewrite s5 properly.</think>



<｜DSML｜tool_calls>
<｜DSML｜invoke name="bash">
<｜DSML｜parameter name="command" string="true">cat /home/takuya/ghq/github.com/dev-komenzar/promptnotes/.ori/scenarios/s5-delete-undo-in-window/wdio.conf.ts