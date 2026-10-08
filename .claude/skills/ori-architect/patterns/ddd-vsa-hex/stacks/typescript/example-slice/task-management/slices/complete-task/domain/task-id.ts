import { err, ok, type Result } from "../../../shared/types/result.js";

declare const TaskIdBrand: unique symbol;
export type TaskId = string & { readonly [TaskIdBrand]: true };

export class TaskIdError extends Error {
  override readonly name = "TaskIdError";
}

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function taskId(raw: string): Result<TaskId, TaskIdError> {
  if (!UUID_RE.test(raw)) {
    return err(new TaskIdError(`invalid TaskId: ${raw}`));
  }
  return ok(raw as TaskId);
}
