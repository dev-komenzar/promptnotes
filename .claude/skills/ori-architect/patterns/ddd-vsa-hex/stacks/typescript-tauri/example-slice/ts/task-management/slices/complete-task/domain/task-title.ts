import { err, ok, type Result } from "../../../shared/types/result.js";

declare const TaskTitleBrand: unique symbol;
export type TaskTitle = string & { readonly [TaskTitleBrand]: true };

export class TaskTitleError extends Error {
  override readonly name = "TaskTitleError";
}

const MAX_LEN = 200;

export function taskTitle(raw: string): Result<TaskTitle, TaskTitleError> {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    return err(new TaskTitleError("title must not be empty"));
  }
  if (trimmed.length > MAX_LEN) {
    return err(new TaskTitleError(`title must be <= ${MAX_LEN} chars`));
  }
  return ok(trimmed as TaskTitle);
}
