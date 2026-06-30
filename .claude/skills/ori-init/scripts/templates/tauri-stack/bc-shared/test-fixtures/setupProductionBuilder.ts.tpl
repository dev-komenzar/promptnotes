// Production-wired IPC builder for Slice DoD boundary tests.
//
// Returns a handler suitable for `@tauri-apps/api/mocks#mockIPC`, wired
// to each slice's PRODUCTION adapter set (no fakes / no stubs). DoD
// rule 3 (.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md)
// requires boundary tests to construct slices via this single entry —
// per-slice fake builders MUST NOT be used to satisfy DoD.
//
// Usage (inside a slice's tests/):
//   import { mockIPC } from "@tauri-apps/api/mocks";
//   import { setupProductionBuilder } from "__BC_NAME__/shared/test-fixtures";
//   beforeEach(() => mockIPC(setupProductionBuilder()));
//
// When a new slice ships, add its command dispatch arm in the switch
// below so boundary tests can exercise it through the bindings layer.

export type IpcHandler = (cmd: string, payload: unknown) => unknown;

export function setupProductionBuilder(): IpcHandler {
  return (cmd: string, _payload: unknown) => {
    switch (cmd) {
      // case "complete_task_cmd":
      //   return /* call production adapter wiring here */;
      default:
        throw new Error(
          `setupProductionBuilder: no production handler registered for "${cmd}". ` +
            `Add a case in __BC_NAME__/shared/test-fixtures/setupProductionBuilder.ts.`,
        );
    }
  };
}
