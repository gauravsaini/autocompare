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
  /**
   * ODD Phase 8: inclusive date-range filter. Accepts full ISO timestamps
   * (`started_at`/`at`) or plain `YYYY-MM-DD` prefixes; comparison is
   * lexicographic on the shared prefix, so either form composes.
   */
  from?: string;
  to?: string;
}

/**
 * ODD Phase 12 defaults review (comment-only): 50 keeps ad-hoc queries (and
 * the e2e introspection hook) bounded to a fraction of the 1024-record ring,
 * so a query can never force a full-buffer scan into a hot path. No §11
 * budget constrains this value; it is a guardrail, not a sampling rate.
 */
const DEFAULT_LIMIT = 50;

function isSpanRecord(record: SinkRecord): boolean {
  return record["kind"] === "span";
}

/**
 * List recent spans, newest first. Filters are exact-match on `op`,
 * `status`, `trace_id`, `component`. `from`/`to` bound the record timestamp
 * (`started_at`, falling back to `at`) inclusively. `limit` defaults to 50.
 */
export function listRecentSpans(query?: SpanQuery): SinkRecord[] {
  const limit = query?.limit ?? DEFAULT_LIMIT;
  const all = getBufferedRecords();
  return filterSpanRecords(all, query).slice(0, limit);
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

// ---------------------------------------------------------------------------
// ODD Phase 8: offline JSONL parsing + extended filtering (additive).
//
// Everything above is untouched in behaviour: `listRecentSpans` still reads
// the in-memory ring buffer and now delegates to `filterSpanRecords`, which
// preserves its exact-match semantics and newest-first order while adding
// the `from`/`to` date-range filter. The helpers below parse JSONL text
// (e.g. `<support-dir>/telemetry/YYYY-MM-DD/*.jsonl` day files) so the same
// query surface works offline without any file I/O imports — callers read
// files and hand the text to `parseJsonlRecords` / `queryJsonlText`.
// ---------------------------------------------------------------------------

/** Record timestamp: `started_at` (span) or `at` (event), when present. */
export function recordTimestamp(record: SinkRecord): string | null {
  const started = record["started_at"];
  if (typeof started === "string" && started.length > 0) return started;
  const at = record["at"];
  if (typeof at === "string" && at.length > 0) return at;
  return null;
}

/** True when `timestamp` lies inside the inclusive `[from, to]` range. */
export function inDateRange(
  timestamp: string | null,
  from?: string,
  to?: string,
): boolean {
  if (from === undefined && to === undefined) return true;
  if (timestamp === null) return false;
  if (from !== undefined && timestamp < from) return false;
  if (to !== undefined && timestamp > to) return false;
  return true;
}

/**
 * Parse JSONL text into records. Malformed lines are skipped and counted in
 * `skipped` — parsing never throws, mirroring the sink's never-throw rule.
 */
export function parseJsonlRecords(text: string): {
  records: SinkRecord[];
  skipped: number;
} {
  const records: SinkRecord[] = [];
  let skipped = 0;
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.length === 0) continue;
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)) {
        records.push(parsed as SinkRecord);
      } else {
        skipped += 1;
      }
    } catch {
      skipped += 1;
    }
  }
  return { records, skipped };
}

/**
 * Filter span records, newest-first. Exact-match on `op`, `status`,
 * `trace_id`, `component`; inclusive `from`/`to` over the record timestamp.
 * Non-span records never match.
 */
export function filterSpanRecords(
  records: SinkRecord[],
  query?: SpanQuery,
): SinkRecord[] {
  const out: SinkRecord[] = [];
  for (let i = records.length - 1; i >= 0; i -= 1) {
    const record = records[i];
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
    if (!inDateRange(recordTimestamp(record), query?.from, query?.to)) {
      continue;
    }
    out.push(record);
  }
  return out;
}

/**
 * Offline query over JSONL text: parse one (or concatenated) day files and
 * apply the full `SpanQuery` surface, newest-first, honouring `limit`.
 */
export function queryJsonlText(text: string, query?: SpanQuery): SinkRecord[] {
  const { records } = parseJsonlRecords(text);
  const filtered = filterSpanRecords(records, query);
  const limit = query?.limit ?? DEFAULT_LIMIT;
  return filtered.slice(0, limit);
}
