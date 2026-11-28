/**
 * ODD Phase 2 Leaf B — trace query stub (reads the sink ring buffer).
 *
 * Stable query surface (§9): `listRecentSpans` mirrors the future
 * `autocompare trace list` filters (`--op`, `--status`). Phase 8 backs
 * this with the JSONL sink dir + `trace show/errors/stats`; until then
 * everything here reads the in-memory buffer, so it is suitable for
 * in-app/e2e use and agent validation loops.
 */

import { getBufferedRecords, type SinkRecord } from "./sink.ts";

export interface SpanQuery {
  op?: string;
  status?: "ok" | "error";
  trace_id?: string;
  component?: string;
  limit?: number;
}

const DEFAULT_LIMIT = 50;

function isSpanRecord(record: SinkRecord): boolean {
  return record["kind"] === "span";
}

/**
 * List recent spans, newest first. Filters are exact-match on `op`,
 * `status`, `trace_id`, `component`. `limit` defaults to 50.
 */
export function listRecentSpans(query?: SpanQuery): SinkRecord[] {
  const limit = query?.limit ?? DEFAULT_LIMIT;
  const all = getBufferedRecords();
  const out: SinkRecord[] = [];
  for (let i = all.length - 1; i >= 0; i -= 1) {
    const record = all[i];
    if (!isSpanRecord(record)) continue;
    if (query?.op !== undefined && record["op"] !== query.op) continue;
    if (query?.status !== undefined && record["status"] !== query.status) {
      continue;
    }
    if (
      query?.trace_id !== undefined &&
      record["trace_id"] !== query.trace_id
    ) {
      continue;
    }
    if (
      query?.component !== undefined &&
      record["component"] !== query.component
    ) {
      continue;
    }
    out.push(record);
    if (out.length >= limit) break;
  }
  return out;
}

/** Newest error span for a trace (or overall when `trace_id` is omitted). */
export function latestErrorSpan(traceId?: string): SinkRecord | null {
  const found = listRecentSpans({
    status: "error",
    trace_id: traceId,
    limit: 1,
  });
  return found.length > 0 ? found[0] : null;
}

/** Count buffered spans per `op` (lightweight `trace stats` precursor). */
export function countSpansByOp(): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const record of getBufferedRecords()) {
    if (!isSpanRecord(record)) continue;
    const op = record["op"];
    if (typeof op !== "string") continue;
    counts[op] = (counts[op] ?? 0) + 1;
  }
  return counts;
}
