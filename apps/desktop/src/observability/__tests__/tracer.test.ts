import { describe, it, beforeEach } from "node:test";
import assert from "node:assert/strict";
import {
  endSpan,
  setTelemetryEnabled,
  startChildSpan,
  startRootSpan,
} from "../tracer.ts";
import { clearBuffer, getBufferedRecords } from "../sink.ts";
import { countSpansByOp, listRecentSpans } from "../query.ts";

const TRACE_ID_RE = /^[0-9a-f]{32}$/;
const SPAN_ID_RE = /^[0-9a-f]{16}$/;

describe("observability tracer (ODD Phase 2 Leaf B)", () => {
  beforeEach(() => {
    clearBuffer();
    setTelemetryEnabled(true);
  });

  it("starts a schema-valid root span", () => {
    const span = startRootSpan({
      op: "text.compare",
      component: "ts.ipc/text",
      view_id: "text",
      session_id: "sess_1",
    });
    assert.match(span.trace_id, TRACE_ID_RE);
    assert.match(span.span_id, SPAN_ID_RE);
    assert.equal(span.parent_span_id, null);
    assert.equal(span.op, "text.compare");

    const record = endSpan(span);
    assert.ok(record !== null);
    assert.equal(record["schema_version"], 1);
    assert.equal(record["kind"], "span");
    assert.equal(record["op"], "text.compare");
    assert.equal(record["component"], "ts.ipc/text");
    assert.equal(record["view_id"], "text");
    assert.equal(record["session_id"], "sess_1");
    assert.equal(record["status"], "ok");
    assert.equal(record["redaction"], "v1-default");
    assert.ok(typeof record["started_at"] === "string");
    assert.ok(typeof record["duration_ns"] === "number");
    assert.ok((record["duration_ns"] as number) >= 0);
  });

  it("span.set merges snake_case attributes", () => {
    const span = startRootSpan({ op: "sync.apply" });
    span.set({ plannedCopies: 3, dryRun: false, hunk_count: 7 });
    const record = endSpan(span);
    assert.ok(record !== null);
    const attrs = record["attributes"] as Record<string, unknown>;
    assert.equal(attrs["planned_copies"], 3);
    assert.equal(attrs["dry_run"], false);
    assert.equal(attrs["hunk_count"], 7);
    assert.ok(!("plannedCopies" in attrs));
  });

  it("span.error marks the span failed with truncated detail", () => {
    const span = startRootSpan({ op: "text.compare" });
    span.error({
      kind: "invoke_failed",
      message: "x".repeat(500),
      component: "ts.ipc/client",
    });
    const record = endSpan(span);
    assert.ok(record !== null);
    assert.equal(record["status"], "error");
    const attrs = record["attributes"] as Record<string, unknown>;
    assert.equal(attrs["error_kind"], "invoke_failed");
    assert.equal(attrs["error_component"], "ts.ipc/client");
    assert.ok(
      (attrs["error_message"] as string).length <= 300,
      "error message capped at 300 chars",
    );
  });

  it("child spans continue the trace with parent linkage", () => {
    const root = startRootSpan({ op: "text.compare" });
    const child = startChildSpan(root, {
      op: "text.compare",
      component: "rs.cmd/text",
    });
    assert.equal(child.trace_id, root.trace_id);
    assert.equal(child.parent_span_id, root.span_id);
    assert.notEqual(child.span_id, root.span_id);
    endSpan(child);
    endSpan(root);

    const spans = listRecentSpans({ op: "text.compare" });
    assert.equal(spans.length, 2);
    const childRecord = spans.find((s) => s["span_id"] === child.span_id);
    assert.equal(childRecord?.["parent_span_id"], root.span_id);
  });

  it("sampled-out ok spans are dropped but errors are always kept", () => {
    const dropped = startRootSpan({ op: "ui.tab_switch", sampled: false });
    assert.equal(endSpan(dropped), null);

    const kept = startRootSpan({ op: "ui.tab_switch", sampled: false });
    kept.error({ kind: "render_failed", message: "boom" });
    const record = endSpan(kept);
    assert.ok(record !== null);
    assert.equal(record["status"], "error");
    assert.equal(getBufferedRecords().length, 1);
  });

  it("telemetry off emits nothing and changes no behaviour", () => {
    setTelemetryEnabled(false);
    const span = startRootSpan({ op: "text.compare" });
    span.set({ hunk_count: 1 });
    span.error({ kind: "x", message: "y" });
    assert.equal(endSpan(span), null);
    assert.equal(getBufferedRecords().length, 0);
    assert.deepEqual(countSpansByOp(), {});
  });

  it("endSpan is idempotent (double close returns null)", () => {
    const span = startRootSpan({ op: "folder.compare" });
    assert.ok(endSpan(span) !== null);
    assert.equal(endSpan(span), null);
    assert.equal(getBufferedRecords().length, 1);
  });
});
