// Presentation layer for the complete-task slice.
// Owns the view model and pure render for a single task row.
// Slice-internal rule: presentation -> [application, domain].

import type { Task } from "../domain/task.js";

export interface TaskCardProps {
  readonly id: string;
  readonly title: string;
  readonly completed: boolean;
}

export function toTaskCardProps(task: Task): TaskCardProps {
  return {
    id: task.id,
    title: task.title,
    completed: task.status === "completed",
  };
}

export function renderTaskCard(props: TaskCardProps): string {
  const mark = props.completed ? "[x]" : "[ ]";
  return `${mark} ${props.title}`;
}
