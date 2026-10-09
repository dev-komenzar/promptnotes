// Application layer for the complete-task slice (TS side).
// Orchestrates the TS-side domain function; depends only on `../domain/*`.
// (In a real Tauri app this layer would delegate to the Rust backend via
// the tauri-specta-generated bindings — see `task-management/shared/ipc/`.)

import { isOk } from "../../../shared/types/result.js";
import type { TaskCompleted } from "../domain/events.js";
import { completeTask, type Task } from "../domain/task.js";

export interface CompleteTaskResult {
  readonly nextTask: Task;
  readonly events: readonly TaskCompleted[];
}

export class CompleteTaskActionError extends Error {
  override readonly name = "CompleteTaskActionError";
}

export function completeTaskAction(
  task: Task,
  now?: () => Date,
): CompleteTaskResult | CompleteTaskActionError {
  const r = completeTask(task, now);
  if (!isOk(r)) {
    return new CompleteTaskActionError(r.error.message);
  }
  return {
    nextTask: r.value.state,
    events: r.value.events,
  };
}
