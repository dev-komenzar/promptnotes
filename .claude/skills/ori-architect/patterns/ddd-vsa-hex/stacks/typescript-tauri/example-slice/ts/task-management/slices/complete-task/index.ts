// Public API for the `complete-task` slice.
//
// Anything not re-exported here is slice-internal; cross-slice imports must
// go through this file (enforced by eslint-plugin-boundaries via the
// architecture adapter).

// Domain
export type { Task, CommandResult, TaskStatus } from "./domain/task.js";
export { createTask, completeTask, TaskStateError } from "./domain/task.js";
export type { TaskId } from "./domain/task-id.js";
export { taskId, TaskIdError } from "./domain/task-id.js";
export type { TaskTitle } from "./domain/task-title.js";
export { taskTitle, TaskTitleError } from "./domain/task-title.js";
export type {
  TaskArchived,
  TaskCompleted,
  TaskCreated,
  TaskEvent,
} from "./domain/events.js";

// Application
export {
  completeTaskAction,
  CompleteTaskActionError,
} from "./application/complete-task.js";
export type { CompleteTaskResult } from "./application/complete-task.js";

// Presentation
export {
  renderTaskCard,
  toTaskCardProps,
} from "./presentation/task-card.js";
export type { TaskCardProps } from "./presentation/task-card.js";
