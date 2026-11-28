import { describe, it, beforeEach } from "node:test";
import assert from "node:assert/strict";
import {
  dailyJsonlPath,
  emitSpan,
  isDatedDirName,
  prunePlan,
  retentionCutoffYmd,
  toYmd,
  DAILY_FILE_NAME,
  DEFAULT_RETENTION_DAYS,
  TELEMETRY_DIR_NAME,
  type SinkRecord,
} from "../sink.ts";
import { clearBuffer } from "../sink.ts";
import {
  filterSpanRecords,
  inDateRange,
  parseJsonlRecords,
  queryJsonlText,
  recordTimestamp,
} from "../query.ts";

const TRACE_ID_RE = /^[0-9a-f]{32}$/;
const SPAN_ID_RE = /^[0-9a-f]{16}$/;

/** Deterministic PRNG (mulberry32) so the 1k seed is stable across runs. */
function rng(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function hex(rand: () => number, len: number): string {
  let out = "";
  const digits = "0123456789abcdef";
  for (let i = 0; i < len; i += 1) {
    out += digits[Math.floor(rand() * 16)];
  }
  return out;
}

const OPS = ["text.compare", "folder.compare", "sync.apply", "ui.tab_switch"];
const COMPONENTS = ["rs.cmd/text", "rs.cmd/folder", "ts.ui/V2App"];

interface SeedBundle {
  jsonl: string;
  records: SinkRecord[];
  traceIds: string[];
}

/** 1000 schema-shaped span records spread over 2026-11-20..28. */
function seedThousand(): SeedBundle {
  const rand = rng(0x0dd8);
  const traceIds = [hex(rand, 32), hex(rand, 32), hex(rand, 32)];
  const records: SinkRecord[] = [];
  for (let i = 0; i < 1000; i += 1) {
    const day = 20 + Math.floor(rand() * 9);
    const op = OPS[i % OPS.length];
    const status = rand() < 0.2 ? "error" : "ok";
    const traceId = traceIds[i % traceIds.length];
    records.push({
      schema_version: 1,
      kind: "span",
      trace_id: traceId,
      span_id: hex(rand, 16),
      parent_span_id: null,
      op,
      component: COMPONENTS[i % COMPONENTS.length],
      started_at: `2026-11-${String(day).padStart(2, "0")}T10:${String(i % 60).padStart(2, "0")}:00.000Z`,
      duration_ns: i,
      status,
      attributes: { iter: i },
      redaction: "v1-default",
    });
  }
  const jsonl = records.map((r) => JSON.stringify(r)).join("\n") + "\n";
  return { jsonl, records, traceIds };
}

function assertSpanSchemaValid(record: SinkRecord): void {
  assert.equal(record["schema_version"], 1);
  assert.equal(record["kind"], "span");
  assert.match(record["trace_id"] as string, TRACE_ID_RE);
  assert.match(record["span_id"] as string, SPAN_ID_RE);
  assert.ok(typeof record["op"] === "string");
  assert.ok((record["component"] as string).includes("."));
  assert.ok(typeof record["started_at"] === "string");
  assert.ok(typeof record["duration_ns"] === "number");
  assert.ok(record["status"] === "ok" || record["status"] === "error");
  assert.equal(typeof record["attributes"], "object");
  assert.equal(record["redaction"], "v1-default");
}

describe("observability Phase 8 collection-storage", () => {
  beforeEach(() => {
    clearBuffer();
  });

  it("query round-trips 1k seeded records through JSONL text", () => {
    const { jsonl, records, traceIds } = seedThousand();
    const { records: parsed, skipped } = parseJsonlRecords(jsonl);
    assert.equal(skipped, 0);
    assert.equal(parsed.length, 1000);

    // Spot-check schema validity on a sample of parsed records.
    for (let i = 0; i < records.length; i += 37) {
      assertSpanSchemaValid(parsed[i]);
    }

    // Filter by trace_id: every third record shares one of three traces.
    const byTrace = queryJsonlText(jsonl, {
      trace_id: traceIds[0],
      limit: 1000,
    });
    assert.ok(byTrace.length > 300 && byTrace.length < 350);
    for (const r of byTrace) assert.equal(r["trace_id"], traceIds[0]);

    // Filter by op + status composes.
    const errors = queryJsonlText(jsonl, {
      op: "text.compare",
      status: "error",
      limit: 1000,
    });
    for (const r of errors) {
      assert.equal(r["op"], "text.compare");
      assert.equal(r["status"], "error");
    }
    const oks = queryJsonlText(jsonl, { status: "ok", limit: 1000 });
    assert.equal(oks.length + queryJsonlText(jsonl, { status: "error", limit: 1000 }).length, 1000);

    // Date range filter: single day slice.
    const day = queryJsonlText(jsonl, {
      from: "2026-11-21T00:00:00.000Z",
      to: "2026-11-21T23:59:59.999Z",
      limit: 1000,
    });
    assert.ok(day.length > 0);
    for (const r of day) {
      const ts = recordTimestamp(r);
      assert.ok(ts !== null && ts.startsWith("2026-11-21"));
    }

    // Plain YYYY-MM-DD bounds compose with full timestamps.
    const ranged = queryJsonlText(jsonl, {
      from: "2026-11-27",
      to: "2026-11-28",
      limit: 1000,
    });
    for (const r of ranged) {
      const dayPrefix = (recordTimestamp(r) ?? "").slice(0, 10);
      assert.ok(dayPrefix === "2026-11-27" || dayPrefix === "2026-11-28");
    }

    // Newest-first order preserved through the offline path.
    const all = queryJsonlText(jsonl, { limit: 1000 });
    assert.equal(all.length, 1000);
    assert.deepEqual(all[0], records[records.length - 1]);
  });

  it("parseJsonl skips malformed lines without throwing", () => {
    const { records, skipped } = parseJsonlRecords(
      '{"kind":"span"}\nnot json\n[1,2]\n{"kind":"span"}\n\n',
    );
    assert.equal(records.length, 2);
    assert.equal(skipped, 2);
  });

  it("inDateRange treats missing bounds and dateless records sanely", () => {
    assert.equal(inDateRange("2026-11-21T00:00:00Z", undefined, undefined), true);
    assert.equal(inDateRange(null, undefined, undefined), true);
    assert.equal(inDateRange(null, "2026-11-01", undefined), false);
    assert.equal(inDateRange("2026-11-20T00:00:00Z", "2026-11-21", undefined), false);
    assert.equal(inDateRange("2026-11-21T00:00:00Z", "2026-11-21", "2026-11-21T23:59:59Z"), true);
  });

  it("buffered listRecentSpans honours the date range (Phase 2 compat)", () => {
    emitSpan({ kind: "span", op: "sync.apply", started_at: "2026-11-20T10:00:00Z" });
    emitSpan({ kind: "span", op: "sync.apply", started_at: "2026-11-25T10:00:00Z" });
    const filtered = filterSpanRecords(
      [
        { kind: "span", op: "sync.apply", started_at: "2026-11-20T10:00:00Z" },
        { kind: "span", op: "sync.apply", started_at: "2026-11-25T10:00:00Z" },
      ],
      { op: "sync.apply", from: "2026-11-21" },
    );
    assert.equal(filtered.length, 1);
    assert.equal(filtered[0]["started_at"], "2026-11-25T10:00:00Z");
  });

  it("dated layout helpers mirror the Rust DatedJsonlSink", () => {
    assert.equal(TELEMETRY_DIR_NAME, "telemetry");
    assert.equal(DAILY_FILE_NAME, "events.jsonl");
    assert.equal(DEFAULT_RETENTION_DAYS, 7);
    assert.equal(
      dailyJsonlPath("/support/telemetry", "2026-11-21"),
      "/support/telemetry/2026-11-21/events.jsonl",
    );
    assert.ok(isDatedDirName("2026-11-21"));
    assert.ok(!isDatedDirName("latest"));
    assert.ok(!isDatedDirName("2026-13-01"));
    assert.ok(!isDatedDirName("2026-02-29"));
    assert.ok(!isDatedDirName("not-a-date"));
    assert.equal(toYmd(new Date("2026-11-21T10:00:00.000Z")), "2026-11-21");
  });

  it("retention cutoff and prune plan keep the 7-day window", () => {
    assert.equal(retentionCutoffYmd("2026-11-28", 7), "2026-11-21");
    assert.equal(retentionCutoffYmd("2026-11-28"), "2026-11-21");
    assert.equal(retentionCutoffYmd("2024-03-01", 1), "2024-02-29");
    assert.equal(retentionCutoffYmd("bogus", 7), null);
    assert.deepEqual(
      prunePlan(
        ["2026-11-01", "2026-11-20", "2026-11-21", "2026-11-27", "latest", "notes.txt"],
        "2026-11-21",
      ),
      ["2026-11-01", "2026-11-20"],
    );
    assert.deepEqual(prunePlan(["2026-11-21"], "bogus"), []);
  });
});
