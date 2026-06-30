// Production-wired IPC builder for Slice DoD boundary tests.
//
// Returns a handler suitable for `@tauri-apps/api/mocks#mockIPC`, wired to
// each slice's PRODUCTION adapter set (no fakes / no stubs at this layer).
// Slice DoD rule 3 (.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md):
// boundary tests MUST construct the slice via this single entry — per-slice
// fake builders MUST NOT be used to satisfy DoD.
//
// Usage (inside a slice's tests/dod.test.ts):
//   import { mockIPC } from "@tauri-apps/api/mocks";
//   import {
//     setupProductionBuilder,
//     seedOpenTask,
//   } from "../../../shared/test-fixtures";
//   beforeEach(() => mockIPC(setupProductionBuilder()));
//
// When a new slice ships, add its command dispatch arm in `setupProductionBuilder`
// so boundary tests can exercise it through the bindings layer.

import { taskId, type TaskId } from "../../slices/complete-task/domain/task-id.js";
import { taskTitle } from "../../slices/complete-task/domain/task-title.js";
import {
  createTask,
  completeTask,
  type Task,
} from "../../slices/complete-task/domain/task.js";
import { isOk } from "../types/result.js";
import type { CompleteTaskResult } from "../ipc/bindings.js";

export type IpcHandler = (cmd: string, payload: unknown) => unknown;

// In-memory store that mirrors the Rust-side `MemoryRepo` for the duration of
// a single test. Reset across tests via `clearProductionStore()` in
// `beforeEach` (or by reconstructing the builder).
const store = new Map<string, Task>();

export function clearProductionStore(): void {
  store.clear();
}

// Seed an open task using the slice's production smart constructors. Tests
// use this to populate the store before invoking `complete_task_cmd`. Mirrors
// the future `create_task_cmd` slice that would normally seed via IPC.
export function seedOpenTask(rawId: string, rawTitle: string): TaskId {
  const id = taskId(rawId);
  const title = taskTitle(rawTitle);
  if (!isOk(id) || !isOk(title)) {
    throw new Error(`seedOpenTask: invalid input (${rawId}, ${rawTitle})`);
  }
  const created = createTask(id.value, title.value);
  store.set(id.value, created.state);
  return id.value;
}

export function setupProductionBuilder(): IpcHandler {
  clearProductionStore();
  return (cmd: string, payload: unknown) => {
    switch (cmd) {
      case "complete_task_cmd": {
        const { id: raw } = (payload ?? {}) as { id?: string };
        if (typeof raw !== "string") {
          throw new Error("complete_task_cmd: missing string `id`");
        }
        const parsed = taskId(raw);
        if (!isOk(parsed)) {
          throw new Error(`complete_task_cmd: invalid TaskId (${raw})`);
        }
        const found = store.get(parsed.value);
        if (!found) {
          throw new Error(`complete_task_cmd: task ${parsed.value} not found`);
        }
        const r = completeTask(found);
        if (!isOk(r)) {
          throw new Error(`complete_task_cmd: ${r.error.message}`);
        }
        store.set(parsed.value, r.value.state);
        const out: CompleteTaskResult = {
          id: parsed.value,
          completed: r.value.state.status === "completed",
        };
        return out;
      }
      // case "<next_slice>_cmd":
      //   return /* dispatch production application use-case here */;
      default:
        throw new Error(
          `setupProductionBuilder: no production handler registered for "${cmd}". ` +
            `Add a case in task-management/shared/test-fixtures/setupProductionBuilder.ts.`,
        );
    }
  };
}
