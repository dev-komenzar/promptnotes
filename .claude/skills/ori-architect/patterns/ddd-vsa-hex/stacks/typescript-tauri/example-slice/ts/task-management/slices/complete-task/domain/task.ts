import { err, ok, type Result } from "../../../shared/types/result.js";
import type { TaskCompleted, TaskCreated } from "./events.js";
import type { TaskId } from "./task-id.js";
import type { TaskTitle } from "./task-title.js";

// 3-value lifecycle: open → completed → archived. The archive transition is
// owned by a sibling slice (archive-task); this slice can read but never
// produce the "archived" state.
export type TaskStatus = "open" | "completed" | "archived";

export interface Task {
  readonly id: TaskId;
  readonly title: TaskTitle;
  readonly status: TaskStatus;
}

export class TaskStateError extends Error {
  override readonly name = "TaskStateError";
}

export interface CommandResult<TState, TEvent> {
  readonly state: TState;
  readonly events: readonly TEvent[];
}

export function createTask(
  id: TaskId,
  title: TaskTitle,
  now: () => Date = () => new Date(),
): CommandResult<Task, TaskCreated> {
  const state: Task = { id, title, status: "open" };
  const event: TaskCreated = {
    name: "TaskCreated",
    occurredAt: now(),
    payload: { id, title },
  };
  return { state, events: [event] };
}

export function completeTask(
  state: Task,
  now: () => Date = () => new Date(),
): Result<CommandResult<Task, TaskCompleted>, TaskStateError> {
  if (state.status !== "open") {
    return err(
      new TaskStateError(
        `task ${state.id} is ${state.status}, only open tasks can be completed`,
      ),
    );
  }
  const next: Task = { ...state, status: "completed" };
  const event: TaskCompleted = {
    name: "TaskCompleted",
    occurredAt: now(),
    payload: { id: state.id },
  };
  return ok({ state: next, events: [event] });
}
