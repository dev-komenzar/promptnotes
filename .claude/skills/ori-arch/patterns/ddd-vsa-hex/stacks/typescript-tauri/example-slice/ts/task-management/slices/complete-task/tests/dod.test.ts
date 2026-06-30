// Slice DoD boundary test for `complete-task`.
//
// Reference implementation per the DoD contract declared in
//   .apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md ("Slice Definition of Done")
//   .apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/architecture.md.tpl
//     ("Test Contract (typescript-tauri instantiation)")
//
// What this file demonstrates (and what AI agents should mirror in new slices):
//
//   1. Imports the command surface ONLY through the tauri-specta-generated
//      bindings: `task-management/shared/ipc/bindings` (DoD rule 2).
//      Importing `../application/*` directly from this file is a DoD violation.
//
//   2. Constructs the slice via the PRODUCTION wiring:
//      `setupProductionBuilder()` from `shared/test-fixtures` (DoD rule 3).
//      Per-slice fake builders MUST NOT be used to satisfy DoD.
//
//   3. Exercises the slice through `commands.completeTaskCmd(...)` which is
//      routed by `mockIPC` to the production handler registered above.
//      Inside a real `pnpm tauri dev` run the same call would be served by
//      the Rust `#[tauri::command]` declared in
//      `apps/<app>/src-tauri/src/task_management/slices/complete_task/commands.rs`.
//
// Unit-level domain tests (fast-check / pure functions) live in
// `task.test.ts` next to this file and DO NOT count toward DoD.

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";

import { commands } from "../../../shared/ipc/bindings.js";
import {
  clearProductionStore,
  seedOpenTask,
  setupProductionBuilder,
} from "../../../shared/test-fixtures/index.js";

const SAMPLE_ID = "1f8b2a02-1111-4222-8333-444455556666";

describe("slice:complete-task DoD (boundary)", () => {
  beforeEach(() => {
    mockIPC(setupProductionBuilder());
  });

  afterEach(() => {
    clearMocks();
    clearProductionStore();
  });

  it("completes a seeded open task via the tauri-specta surface", async () => {
    seedOpenTask(SAMPLE_ID, "buy milk");

    const result = await commands.completeTaskCmd({ id: SAMPLE_ID });

    expect(result.id).toBe(SAMPLE_ID);
    expect(result.completed).toBe(true);
  });

  it("rejects a non-existent task with a NotFound-shaped error", async () => {
    await expect(
      commands.completeTaskCmd({ id: SAMPLE_ID }),
    ).rejects.toThrow(/not found/i);
  });

  it("rejects an invalid TaskId at the boundary (Smart Constructor parses raw input)", async () => {
    await expect(
      commands.completeTaskCmd({ id: "not-a-uuid" }),
    ).rejects.toThrow(/invalid taskid/i);
  });

  it("refuses to complete an already-completed task", async () => {
    seedOpenTask(SAMPLE_ID, "buy milk");
    await commands.completeTaskCmd({ id: SAMPLE_ID });

    await expect(
      commands.completeTaskCmd({ id: SAMPLE_ID }),
    ).rejects.toThrow(/already completed|only open tasks/i);
  });
});
