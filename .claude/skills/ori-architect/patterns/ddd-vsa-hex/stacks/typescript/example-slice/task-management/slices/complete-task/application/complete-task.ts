// Application layer for the complete-task slice.
// Orchestrates the domain function; depends only on `../domain/*` and the BC
// shared types (slice-internal rule: application -> [domain]).

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
