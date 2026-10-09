import type { DomainEvent } from "../../../shared/events/event.js";
import type { TaskId } from "./task-id.js";
import type { TaskTitle } from "./task-title.js";

export type TaskCreated = DomainEvent<
  "TaskCreated",
  { readonly id: TaskId; readonly title: TaskTitle }
>;

export type TaskCompleted = DomainEvent<
  "TaskCompleted",
  { readonly id: TaskId }
>;

// Owned by archive-task slice but declared in the shared event union so
// downstream consumers (UI projections, integration tests) can exhaust the
// lifecycle without reaching across slices.
export type TaskArchived = DomainEvent<
  "TaskArchived",
  { readonly id: TaskId }
>;

export type TaskEvent = TaskCreated | TaskCompleted | TaskArchived;
