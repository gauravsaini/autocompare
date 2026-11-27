import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  activeEvents,
  appendEvent,
  canRedoEvent,
  canUndoEvent,
  createEventLog,
  exportEventLogJsonl,
  redoEvent,
  undoEvent,
} from "../utils/sessionEventLog.ts";

describe("sessionEventLog (Phase 5 UI Undo/Redo & JSONL)", () => {
  it("appends, undoes, redoes, and truncates redo tail on branch", () => {
    let log = createEventLog<{ choice: string }>();
    assert.equal(canUndoEvent(log), false);

    log = appendEvent(log, "merge_resolve", "Conflict #1 -> left", { choice: "left" });
    log = appendEvent(log, "merge_resolve", "Conflict #2 -> right", { choice: "right" });
    assert.equal(activeEvents(log).length, 2);
    assert.equal(canUndoEvent(log), true);
    assert.equal(canRedoEvent(log), false);

    const u1 = undoEvent(log);
    log = u1.next;
    assert.equal(u1.undone?.label, "Conflict #2 -> right");
    assert.equal(activeEvents(log).length, 1);
    assert.equal(canRedoEvent(log), true);

    const r1 = redoEvent(log);
    log = r1.next;
    assert.equal(r1.redone?.label, "Conflict #2 -> right");
    assert.equal(activeEvents(log).length, 2);

    log = undoEvent(log).next;
    log = appendEvent(log, "merge_resolve", "Conflict #2 -> base", { choice: "base" });
    assert.equal(canRedoEvent(log), false);

    const jsonl = exportEventLogJsonl(log);
    const lines = jsonl.split("\n");
    assert.equal(lines.length, 2);
    assert.ok(lines[1].includes("Conflict #2 -> base"));
  });
});
