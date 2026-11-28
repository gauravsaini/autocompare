import { describe, it, beforeEach } from "node:test";
import assert from "node:assert/strict";
import {
  childContext,
  fromTraceparent,
  newRootContext,
  parse,
  serialize,
  toTraceparent,
} from "../context.ts";
import { endSpan, setTelemetryEnabled, startRootSpan } from "../tracer.ts";
import { clearBuffer } from "../sink.ts";
import { listRecentSpans } from "../query.ts";

const TRACE_ID_RE = /^[0-9a-f]{32}$/;
const SPAN_ID_RE = /^[0-9a-f]{16}$/;

describe("observability propagation (TS ⇄ Rust TraceContext rules)", () => {
  beforeEach(() => {
    clearBuffer();
    setTelemetryEnabled(true);
  });

  it("__trace envelope serialize/parse round-trips losslessly", () => {
    const ctx = newRootContext();
    const wire = JSON.parse(JSON.stringify(serialize(ctx))) as unknown;
    const back = parse(wire);
    assert.ok(back !== null);
    assert.equal(back.trace_id, ctx.trace_id);
    assert.equal(back.span_id, ctx.span_id);
    assert.equal(back.sampled, ctx.sampled);
  });

  it("child context inherits the trace and links the parent", () => {
    const root = newRootContext();
    const child = childContext(root);
    assert.equal(child.trace_id, root.trace_id);
    assert.equal(child.parent_span_id, root.span_id);
    assert.notEqual(child.span_id, root.span_id);
    assert.equal(child.sampled, root.sampled);
  });

  it("parse rejects malformed or absent input with null", () => {
    assert.equal(parse(null), null);
    assert.equal(parse(undefined), null);
    assert.equal(parse("traceparent-string"), null);
    assert.equal(parse(42), null);
    assert.equal(parse({}), null);
    assert.equal(parse({ trace_id: "xyz", span_id: "00f067aa0ba902b7" }), null);
    assert.equal(
      parse({
        trace_id: "4bf92f3577b34da6a3ce929d0e0e4736",
        span_id: "short",
      }),
      null,
    );
    assert.equal(
      parse({
        trace_id: "4bf92f3577b34da6a3ce929d0e0e4736",
        span_id: "00f067aa0ba902b7",
        parent_span_id: "not-hex",
      }),
      null,
    );
    // Uppercase hex is not canonical wire format.
    assert.equal(
      parse({
        trace_id: "4BF92F3577B34DA6A3CE929D0E0E4736",
        span_id: "00F067AA0BA902B7",
      }),
      null,
    );
  });

  it("absent context mints a fresh root", () => {
    assert.equal(parse(undefined), null);
    const fresh = newRootContext();
    assert.match(fresh.trace_id, TRACE_ID_RE);
    assert.match(fresh.span_id, SPAN_ID_RE);
    assert.equal(fresh.parent_span_id, null);
    assert.equal(fresh.sampled, true);
  });

  it("traceparent header round-trips the sampling flag", () => {
    const sampled = newRootContext({ sampled: true });
    const unsampled = newRootContext({ sampled: false });

    const h1 = toTraceparent(sampled);
    const h2 = toTraceparent(unsampled);
    assert.match(h1, /^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/);
    assert.match(h2, /^00-[0-9a-f]{32}-[0-9a-f]{16}-00$/);

    const back1 = fromTraceparent(h1);
    const back2 = fromTraceparent(h2);
    assert.ok(back1 !== null && back2 !== null);
    assert.equal(back1.trace_id, sampled.trace_id);
    assert.equal(back1.sampled, true);
    assert.equal(back2.sampled, false);

    // Strict rejections: bad version, all-zero ids, garbage.
    assert.equal(fromTraceparent("01-aaa-bbb-01"), null);
    assert.equal(
      fromTraceparent(
        "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
      ),
      null,
    );
    assert.equal(fromTraceparent("not-a-header"), null);
    assert.equal(fromTraceparent(42), null);
  });

  it("tracer continues a trace from an incoming __trace envelope", () => {
    const upstream = newRootContext();
    const envelope = JSON.parse(JSON.stringify(serialize(upstream))) as unknown;

    const span = startRootSpan({
      op: "text.compare",
      component: "rs.cmd/text",
      parent: envelope,
    });
    assert.equal(span.trace_id, upstream.trace_id);
    assert.equal(span.parent_span_id, upstream.span_id);
    const record = endSpan(span);
    assert.ok(record !== null);

    const found = listRecentSpans({ trace_id: upstream.trace_id });
    assert.equal(found.length, 1);
    assert.equal(found[0]["span_id"], span.span_id);
  });

  it("mock-style echo preserves the trace across the IPC boundary", () => {
    // Simulates Phase 3 flow: TS injects __trace, mock/transport echoes it.
    const span = startRootSpan({ op: "sync.apply" });
    const params = { dry_run: true, __trace: span.context() };
    const echoed = JSON.parse(JSON.stringify(params)) as {
      __trace: unknown;
    };
    const echoedCtx = parse(echoed.__trace);
    assert.ok(echoedCtx !== null);
    assert.equal(echoedCtx.trace_id, span.trace_id);
    endSpan(span);

    const found = listRecentSpans({ trace_id: span.trace_id });
    assert.equal(found.length, 1);
  });
});
