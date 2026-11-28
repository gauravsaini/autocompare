/**
 * ODD Phase 2 Leaf B — sinks (console + buffered file-sink stub).
 *
 * Pipeline role (§11): emitters → redaction gate → sampler → bounded
 * channel → batch → JSONL sink. This leaf owns the tail end:
 * - `emitSpan` / `emitEvent`: append to the in-memory ring buffer and
 *   mirror to the console sink when enabled.
 * - `BufferedFileSink`: stub that buffers records in memory; the real
 *   JSONL file layout (`<support-dir>/telemetry/YYYY-MM-DD/*.jsonl`,
 *   rotation/retention) lands in Phase 8.
 *
 * Reliability rules (§11): sink writes never block ops and never throw —
 * overflow degrades to drop-oldest + counter. All entry points are
 * guarded so instrumentation panics are counted, never propagated.
 */

/** Minimal structural view of an emitted record (schema lives in tracer). */
export type SinkRecord = Record<string, unknown>;

/** Ring capacity: one batch window (1s / 512 events, §11) with headroom. */
export const SINK_BUFFER_LIMIT = 1024;

let buffer: SinkRecord[] = [];
let dropCount = 0;
let consoleEnabled = readConsoleFlag();
let instrumentationErrorCount = 0;

function readConsoleFlag(): boolean {
  try {
    if (typeof process !== "undefined" && process.env) {
      const v = process.env["AUTOCOMPARE_TELEMETRY_CONSOLE"];
      if (v === "1" || v === "on" || v === "true") return true;
    }
  } catch {
    // ignore — default off
  }
  return false;
}

export function setConsoleSinkEnabled(enabled: boolean): void {
  consoleEnabled = enabled;
}

export function isConsoleSinkEnabled(): boolean {
  return consoleEnabled;
}

/** Number of records dropped due to ring overflow (monotonic). */
export function getDropCount(): number {
  return dropCount;
}

/** Instrumentation-internal failures caught and counted (never thrown). */
export function getInstrumentationErrorCount(): number {
  return instrumentationErrorCount;
}

export function noteInstrumentationError(): void {
  instrumentationErrorCount += 1;
}

function push(record: SinkRecord): void {
  if (buffer.length >= SINK_BUFFER_LIMIT) {
    buffer.shift();
    dropCount += 1;
  }
  buffer.push(record);
  if (consoleEnabled) {
    try {
      console.log(JSON.stringify(record));
    } catch {
      instrumentationErrorCount += 1;
    }
  }
}

function snapshot(record: SinkRecord): SinkRecord {
  try {
    return JSON.parse(JSON.stringify(record)) as SinkRecord;
  } catch {
    return { ...record };
  }
}

export function emitSpan(record: SinkRecord): void {
  try {
    push(snapshot(record));
  } catch {
    instrumentationErrorCount += 1;
  }
}

export function emitEvent(record: SinkRecord): void {
  try {
    push(snapshot(record));
  } catch {
    instrumentationErrorCount += 1;
  }
}

/** Oldest-first snapshot of the ring buffer (read-only copy). */
export function getBufferedRecords(): SinkRecord[] {
  return buffer.slice();
}

export function clearBuffer(): void {
  buffer = [];
  dropCount = 0;
}

/**
 * Buffered file-sink stub (Phase 8 wires the real JSONL layout).
 * Browser-safe: no `node:fs` import; `flush` is a no-op that reports
 * what WOULD be written so callers and tests can rely on the shape.
 */
export interface FlushReport {
  path: string;
  flushed: number;
  wrote: boolean;
  note: string;
}

export class BufferedFileSink {
  readonly path: string;
  private pending: SinkRecord[] = [];

  constructor(path: string) {
    this.path = path;
  }

  write(record: SinkRecord): void {
    try {
      this.pending.push(snapshot(record));
    } catch {
      instrumentationErrorCount += 1;
    }
  }

  pendingCount(): number {
    return this.pending.length;
  }

  flush(): FlushReport {
    const flushed = this.pending.length;
    this.pending = [];
    return {
      path: this.path,
      flushed,
      wrote: false,
      note: "stub: JSONL file layout lands in Phase 8; records held in memory only",
    };
  }
}
