import { describe, expect, it } from "vitest";
import { isErr, isOk } from "../../../shared/types/result.js";
import { completeTask, createTask } from "../domain/task.js";
import { taskId } from "../domain/task-id.js";
import { taskTitle } from "../domain/task-title.js";

const FIXED_NOW = () => new Date("2026-05-14T00:00:00.000Z");
const SAMPLE_ID = "1f8b2a02-1111-4222-8333-444455556666";

describe("slice:complete-task domain", () => {
  describe("TaskId", () => {
    it("accepts a UUID", () => {
      const r = taskId(SAMPLE_ID);
      expect(isOk(r)).toBe(true);
    });

    it("rejects a non-UUID", () => {
      const r = taskId("not-a-uuid");
      expect(isErr(r)).toBe(true);
    });
  });

  describe("TaskTitle", () => {
    it("accepts a non-empty title and trims it", () => {
      const r = taskTitle("  buy milk  ");
      expect(isOk(r)).toBe(true);
      if (isOk(r)) expect(r.value).toBe("buy milk");
    });

    it("rejects an empty title", () => {
      const r = taskTitle("   ");
      expect(isErr(r)).toBe(true);
    });

    it("rejects an overlong title", () => {
      const r = taskTitle("x".repeat(201));
      expect(isErr(r)).toBe(true);
    });
  });

  describe("createTask / completeTask", () => {
    it("creates a task and emits TaskCreated", () => {
      const id = taskId(SAMPLE_ID);
      const title = taskTitle("buy milk");
      if (!isOk(id) || !isOk(title)) throw new Error("VO smoke failed");

      const result = createTask(id.value, title.value, FIXED_NOW);

      expect(result.state.status).toBe("open");
      expect(result.events).toHaveLength(1);
      expect(result.events[0]?.name).toBe("TaskCreated");
      expect(result.events[0]?.payload).toEqual({
        id: id.value,
        title: title.value,
      });
    });

    it("completes a pending task and emits TaskCompleted", () => {
      const id = taskId(SAMPLE_ID);
      const title = taskTitle("buy milk");
      if (!isOk(id) || !isOk(title)) throw new Error("VO smoke failed");
      const created = createTask(id.value, title.value, FIXED_NOW);

      const completed = completeTask(created.state, FIXED_NOW);

      expect(isOk(completed)).toBe(true);
      if (!isOk(completed)) return;
      expect(completed.value.state.status).toBe("completed");
      expect(completed.value.events[0]?.name).toBe("TaskCompleted");
    });

    it("refuses to complete an already-completed task", () => {
      const id = taskId(SAMPLE_ID);
      const title = taskTitle("buy milk");
      if (!isOk(id) || !isOk(title)) throw new Error("VO smoke failed");
      const created = createTask(id.value, title.value, FIXED_NOW);
      const once = completeTask(created.state, FIXED_NOW);
      if (!isOk(once)) throw new Error("first completeTask failed");

      const twice = completeTask(once.value.state, FIXED_NOW);

      expect(isErr(twice)).toBe(true);
    });
  });
});
