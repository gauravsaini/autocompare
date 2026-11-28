/**
 * ODD Phase 2 Leaf B — TS tracer (hand-rolled, no OTel SDK per §8).
 *
 * Usage (call-site pattern from §13.1; wiring into `src/ipc/client.ts`
 * happens in Phase 3 — this leaf emits only, it changes no call sites):
 *
 * ```ts
 * const span = startRootSpan({ op: "text.copy_hunk", view_id: "text" });
 * span.set({ direction: "l2r" });
 * // ... await work, passing span.context() as `__trace` ...
 * endSpan(span);
 * ```
 *
 * Span records follow `telemetry/schema/v1/span.json` (snake_case,
 * `schema_version: 1`, `redaction: "v1-default"`). Instrumentation never
 * throws: emit is guarded and counted in the sink.
 */

import {
  childContext,
  newRootContext,
  parse,
  shouldKeep,
  type TraceContext,
} from "./context.ts";
import { redactAttributes, type RedactOptions } from "./redact.ts";
import { emitSpan, noteInstrumentationError, type SinkRecord } from "./sink.ts";

export const SCHEMA_VERSION = 1;
export const REDACTION_MARKER = "v1-default";
const ERROR_MESSAGE_BUDGET = 300;

let telemetryOverride: boolean | null = null;

function readKillSwitch(): boolean {
  try {
    if (typeof process !== "undefined" && process.env) {
      if (process.env["AUTOCOMPARE_TELEMETRY"] === "off") return false;
    }
  } catch {
    // ignore — default enabled
  }
  return true;
}

/** Programmatic kill-switch (tests / Phase 3 wiring). */
export function setTelemetryEnabled(enabled: boolean): void {
  telemetryOverride = enabled;
}

export function isTelemetryEnabled(): boolean {
  if (telemetryOverride !== null) return telemetryOverride;
  return readKillSwitch();
}

export type SpanStatus = "ok" | "error";

export interface StartSpanOptions {
  op: string;
  component?: string;
  view_id?: string;
  session_id?: string;
  attributes?: Record<string, unknown>;
  /** Continue this trace: a TraceContext or a `__trace` envelope. */
  parent?: TraceContext | unknown;
  sampled?: boolean;
  redact?: RedactOptions;
}

export interface SpanErrorInfo {
  kind?: string;
  message?: string;
  component?: string;
}

export interface SpanRecord extends SinkRecord {
  schema_version: 1;
  kind: "span";
  trace_id: string;
  span_id: string;
  parent_span_id: string | null;
  op: string;
  component: string;
  view_id?: string;
  session_id?: string;
  started_at: string;
  duration_ns?: number;
  status: SpanStatus;
  attributes: Record<string, unknown>;
  redaction: "v1-default";
}

export interface Span {
  readonly trace_id: string;
  readonly span_id: string;
  readonly parent_span_id: string | null;
  readonly op: string;
  readonly sampled: boolean;
  /** `__trace` envelope to inject into Tauri invoke params. */
  context(): { trace_id: string; span_id: string; sampled: boolean };
  /** Merge (redacted, snake_case) attributes into the span. */
  set(attrs: Record<string, unknown>): void;
  /** Mark the span failed (errors bypass sampling, §7). */
  error(info?: SpanErrorInfo): void;
}

interface SpanState {
  ctx: TraceContext;
  op: string;
  component: string;
  view_id?: string;
  session_id?: string;
  attributes: Record<string, unknown>;
  status: SpanStatus;
  errorKind?: string;
  errorMessage?: string;
  errorComponent?: string;
  startWallClock: number;
  startMono: number;
  ended: boolean;
  redact: RedactOptions;
}

const spanStates = new WeakMap<object, SpanState>();

function nowMono(): number {
  try {
    if (
      typeof performance !== "undefined" &&
      typeof performance.now === "function"
    ) {
      return performance.now();
    }
  } catch {
    // fall through to Date.now
  }
  return Date.now();
}

function resolveContext(
  parent: TraceContext | unknown,
  sampled?: boolean,
): TraceContext {
  if (parent !== null && parent !== undefined) {
    const parsed = parse(parent);
    if (parsed !== null) {
      const child = childContext(parsed);
      if (sampled !== undefined) child.sampled = sampled;
      return child;
    }
    // Malformed envelope → fresh root (caller can detect via parse()).
  }
  return newRootContext(
    sampled !== undefined ? { sampled } : undefined,
  );
}

/** Start a root span (or a child when `opts.parent` carries a context). */
export function startRootSpan(opts: StartSpanOptions): Span {
  const fallback: SpanState = {
    ctx: newRootContext(),
    op: opts.op,
    component: opts.component ?? "ts.ui/unknown",
    view_id: opts.view_id,
    session_id: opts.session_id,
    attributes: {},
    status: "ok",
    startWallClock: Date.now(),
    startMono: nowMono(),
    ended: false,
    redact: { allowPaths: opts.redact?.allowPaths === true },
  };
  let state: SpanState = fallback;
  try {
    const ctx =
      opts.parent !== undefined && opts.parent !== null
        ? resolveContext(opts.parent, opts.sampled)
        : opts.sampled !== undefined
          ? newRootContext({ sampled: opts.sampled })
          : newRootContext();
    state = {
      ...fallback,
      ctx,
      attributes:
        opts.attributes !== undefined
          ? redactAttributes(opts.attributes, fallback.redact)
          : {},
    };
  } catch {
    noteInstrumentationError();
  }

  const captured = state;
  const span: Span = {
    trace_id: captured.ctx.trace_id,
    span_id: captured.ctx.span_id,
    parent_span_id: captured.ctx.parent_span_id,
    op: captured.op,
    sampled: captured.ctx.sampled,
    context() {
      return {
        trace_id: captured.ctx.trace_id,
        span_id: captured.ctx.span_id,
        sampled: captured.ctx.sampled,
      };
    },
    set(attrs: Record<string, unknown>) {
      if (captured.ended) return;
      if (attrs === null || attrs === undefined) return;
      try {
        const clean = redactAttributes(
          attrs as Record<string, unknown>,
          captured.redact,
        );
        for (const [k, v] of Object.entries(clean)) {
          captured.attributes[k] = v;
        }
      } catch {
        noteInstrumentationError();
      }
    },
    error(info?: SpanErrorInfo) {
      if (captured.ended) return;
      try {
        captured.status = "error";
        if (info?.kind !== undefined) {
          captured.errorKind = String(info.kind).slice(0, 128);
        }
        if (info?.component !== undefined) {
          captured.errorComponent = String(info.component).slice(0, 256);
        }
        if (info?.message !== undefined) {
          const clean = redactAttributes(
            { error_message: info.message },
            captured.redact,
          );
          const msg = clean["error_message"];
          captured.errorMessage =
            typeof msg === "string"
              ? msg.slice(0, ERROR_MESSAGE_BUDGET)
              : "[redacted]";
        }
      } catch {
        noteInstrumentationError();
      }
    },
  };
  spanStates.set(span, captured);
  return span;
}

/** Explicit child-span helper (same trace, parent linked). */
export function startChildSpan(
  parent: Span | TraceContext,
  opts: Omit<StartSpanOptions, "parent" | "sampled"> & { sampled?: boolean },
): Span {
  let parentCtx: unknown = null;
  try {
    if (
      parent !== null &&
      typeof parent === "object" &&
      typeof (parent as Span).context === "function"
    ) {
      parentCtx = (parent as Span).context();
    } else {
      const p = parent as TraceContext;
      parentCtx = {
        trace_id: p.trace_id,
        span_id: p.span_id,
        sampled: p.sampled,
      };
    }
  } catch {
    noteInstrumentationError();
  }
  return startRootSpan({ ...opts, parent: parentCtx, sampled: opts.sampled });
}

/**
 * Close a span and emit it. Returns the emitted record, or `null` when
 * the span is sampled out (`ok` + `sampled: false`), already ended,
 * or telemetry is off. Never throws.
 */
export function endSpan(span: Span): SpanRecord | null {
  try {
    if (span === null || span === undefined) return null;
    const state = spanStates.get(span as unknown as object);
    if (state === undefined || state.ended) return null;
    state.ended = true;

    const isError = state.status === "error";
    if (!isTelemetryEnabled()) return null;
    if (!shouldKeep(state.ctx, isError)) return null;

    const elapsedMs = Math.max(0, nowMono() - state.startMono);
    const record: SpanRecord = {
      schema_version: SCHEMA_VERSION,
      kind: "span",
      trace_id: state.ctx.trace_id,
      span_id: state.ctx.span_id,
      parent_span_id: state.ctx.parent_span_id,
      op: state.op,
      component: state.component,
      started_at: new Date(state.startWallClock).toISOString(),
      duration_ns: Math.round(elapsedMs * 1_000_000),
      status: state.status,
      attributes: { ...state.attributes },
      redaction: REDACTION_MARKER,
    };
    if (state.view_id !== undefined) record.view_id = state.view_id;
    if (state.session_id !== undefined) record.session_id = state.session_id;
    if (isError) {
      if (state.errorKind !== undefined) {
        record.attributes["error_kind"] = state.errorKind;
      }
      if (state.errorMessage !== undefined) {
        record.attributes["error_message"] = state.errorMessage;
      }
      if (state.errorComponent !== undefined) {
        record.attributes["error_component"] = state.errorComponent;
      }
    }
    emitSpan(record);
    return record;
  } catch {
    noteInstrumentationError();
    return null;
  }
}
