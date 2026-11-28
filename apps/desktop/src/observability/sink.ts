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

/** Ring capacity: one batch window (1s / 512 events, §11) with headroom.
 *
 * ODD §11 budget note (Phase 12, comment-only): 1024 records × ~2KB/op worst
 * case (the <2MB-per-1k-ops volume ceiling) ≈ 2MB, comfortably inside the 8MB
 * sink-memory cap. Overflow still degrades to drop-oldest + counter.
 */
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

// ---------------------------------------------------------------------------
// ODD Phase 8: dated JSONL layout + retention helpers (additive, pure).
//
// Everything above is untouched. These helpers mirror the Rust
// `DatedJsonlSink` layout (`<support-dir>/telemetry/YYYY-MM-DD/<file>`, 7-day
// default retention) as dependency-free string/set operations, so they stay
// browser-safe: no `node:fs` import. Node callers perform the actual reads,
// writes and directory removals and hand plain data in/out.
// ---------------------------------------------------------------------------

/** Directory name for the dated layout root (`<support-dir>/telemetry`). */
export const TELEMETRY_DIR_NAME = "telemetry";

/** File name used inside each `YYYY-MM-DD` directory. */
export const DAILY_FILE_NAME = "events.jsonl";

/** Default retention window in days (matches the Rust sink). */
export const DEFAULT_RETENTION_DAYS = 7;

const DATED_DIR_RE = /^(\d{4})-(\d{2})-(\d{2})$/;

function isLeapYear(year: number): boolean {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

function daysInMonth(year: number, month: number): number {
  switch (month) {
    case 1:
    case 3:
    case 5:
    case 7:
    case 8:
    case 10:
    case 12:
      return 31;
    case 4:
    case 6:
    case 9:
    case 11:
      return 30;
    default:
      return isLeapYear(year) ? 29 : 28;
  }
}

/** True when `name` is a valid `YYYY-MM-DD` directory name. */
export function isDatedDirName(name: string): boolean {
  const match = DATED_DIR_RE.exec(name);
  if (match === null) return false;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  if (!Number.isInteger(year) || !Number.isInteger(month)) return false;
  if (!Number.isInteger(day) || month < 1 || month > 12) return false;
  return day >= 1 && day <= daysInMonth(year, month);
}

/** Join path segments with `/` (layout helper; not a filesystem call). */
export function joinLayoutPath(...segments: string[]): string {
  return segments
    .map((s) => s.replace(/\/+$/u, ""))
    .filter((s) => s.length > 0)
    .join("/");
}

/** Layout path for a `YYYY-MM-DD` date: `<root>/<ymd>/<file>`. */
export function dailyJsonlPath(
  telemetryRoot: string,
  ymd: string,
  file: string = DAILY_FILE_NAME,
): string {
  return joinLayoutPath(telemetryRoot, ymd, file);
}

/** Format a `Date` as `YYYY-MM-DD` (UTC). */
export function toYmd(date: Date): string {
  const y = date.getUTCFullYear();
  const m = date.getUTCMonth() + 1;
  const d = date.getUTCDate();
  const pad = (n: number): string => (n < 10 ? `0${n}` : `${n}`);
  return `${y}-${pad(m)}-${pad(d)}`;
}

/**
 * Inclusive retention cutoff: dated directories strictly older than the
 * return fall outside a `retentionDays` window ending at `todayYmd`.
 * Returns `null` for malformed input (caller prunes nothing).
 */
export function retentionCutoffYmd(
  todayYmd: string,
  retentionDays: number = DEFAULT_RETENTION_DAYS,
): string | null {
  if (!isDatedDirName(todayYmd)) return null;
  if (!Number.isInteger(retentionDays) || retentionDays < 0) return null;
  const cutoff = new Date(`${todayYmd}T00:00:00.000Z`);
  cutoff.setUTCDate(cutoff.getUTCDate() - retentionDays);
  return toYmd(cutoff);
}

/**
 * Pure prune plan: which entry names to delete given the names present in
 * the telemetry root and an inclusive `keepFromYmd` bound. Only valid
 * `YYYY-MM-DD` directory names strictly older than the bound qualify —
 * files and non-dated entries are never selected.
 */
export function prunePlan(dirNames: string[], keepFromYmd: string): string[] {
  if (!isDatedDirName(keepFromYmd)) return [];
  const doomed: string[] = [];
  for (const name of dirNames) {
    if (!isDatedDirName(name)) continue;
    if (name < keepFromYmd) doomed.push(name);
  }
  return doomed.sort();
}
