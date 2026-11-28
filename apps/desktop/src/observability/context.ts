/**
 * ODD Phase 2 Leaf B — TraceContext (TS mirror).
 *
 * Mirrors the Rust `TraceContext` rules (ODD doc §7):
 * - `trace_id`: 128-bit lowercase hex (32 chars, W3C compatible).
 * - `span_id`: 64-bit lowercase hex (16 chars).
 * - `parent_span_id`: nullable 64-bit hex.
 * - `sampled`: head-based sampling flag that travels with the context.
 *
 * Transports:
 * - Tauri IPC: `__trace` object inside invoke params (`serialize` / `parse`).
 * - HTTP RPC (`autocompare web`): W3C `traceparent` header
 *   (`toTraceparent` / `fromTraceparent`).
 * - Rust → TS: echoed `{ trace_id, span_id }` in the response envelope
 *   (parsed with `parse`; extra envelope fields are ignored).
 *
 * `parse` is strict: malformed or absent input returns `null` so the
 * caller mints a fresh root (and can flag the break). It never throws.
 */

export interface TraceContext {
  trace_id: string;
  span_id: string;
  parent_span_id: string | null;
  sampled: boolean;
}

/** Minimal `__trace` envelope shape shared with Rust over Tauri IPC. */
export interface SerializedTraceContext {
  trace_id: string;
  span_id: string;
  sampled: boolean;
}

const TRACE_ID_RE = /^[0-9a-f]{32}$/;
const SPAN_ID_RE = /^[0-9a-f]{16}$/;
const TRACE_FLAGS_RE = /^[0-9a-f]{2}$/;
const ZERO_TRACE_ID = "00000000000000000000000000000000";
const ZERO_SPAN_ID = "0000000000000000";

function randomHex(bytes: number): string {
  const arr = new Uint8Array(bytes);
  const cryptoObj =
    typeof globalThis !== "undefined" ? globalThis.crypto : undefined;
  if (cryptoObj && typeof cryptoObj.getRandomValues === "function") {
    cryptoObj.getRandomValues(arr);
  } else {
    for (let i = 0; i < bytes; i += 1) {
      arr[i] = Math.floor(Math.random() * 256);
    }
  }
  let out = "";
  for (let i = 0; i < arr.length; i += 1) {
    out += arr[i].toString(16).padStart(2, "0");
  }
  return out;
}

export function generateTraceId(): string {
  return randomHex(16);
}

export function generateSpanId(): string {
  return randomHex(8);
}

export function isValidTraceId(value: unknown): value is string {
  return typeof value === "string" && TRACE_ID_RE.test(value);
}

export function isValidSpanId(value: unknown): value is string {
  return typeof value === "string" && SPAN_ID_RE.test(value);
}

/** Mint a fresh root context (head-based sampling decision happens here). */
export function newRootContext(opts?: { sampled?: boolean }): TraceContext {
  return {
    trace_id: generateTraceId(),
    span_id: generateSpanId(),
    parent_span_id: null,
    sampled: opts?.sampled ?? true,
  };
}

/** Derive a child context: same trace, parent linked, fresh span id. */
export function childContext(parent: TraceContext): TraceContext {
  return {
    trace_id: parent.trace_id,
    span_id: generateSpanId(),
    parent_span_id: parent.span_id,
    sampled: parent.sampled,
  };
}

/** Serialize to the `__trace` envelope shape (Tauri IPC / echo). */
export function serialize(ctx: TraceContext): SerializedTraceContext {
  return {
    trace_id: ctx.trace_id,
    span_id: ctx.span_id,
    sampled: ctx.sampled,
  };
}

/**
 * Parse an incoming `__trace` envelope (or full context object).
 * Returns `null` on absent/malformed input — never throws.
 */
export function parse(input: unknown): TraceContext | null {
  if (input === null || input === undefined) return null;
  if (typeof input !== "object") return null;
  const o = input as Record<string, unknown>;
  const traceId = o["trace_id"];
  const spanId = o["span_id"];
  if (!isValidTraceId(traceId)) return null;
  if (!isValidSpanId(spanId)) return null;
  let parentSpanId: string | null = null;
  const rawParent = o["parent_span_id"];
  if (rawParent !== undefined && rawParent !== null) {
    if (!isValidSpanId(rawParent)) return null;
    parentSpanId = rawParent;
  }
  // Default-dev rule (§7): sampled unless explicitly opted out.
  const sampled = o["sampled"] !== false;
  return {
    trace_id: traceId,
    span_id: spanId,
    parent_span_id: parentSpanId,
    sampled,
  };
}

/** Render a W3C `traceparent` header value (`00-<trace>-<span>-<flags>`). */
export function toTraceparent(ctx: TraceContext): string {
  return `00-${ctx.trace_id}-${ctx.span_id}-${ctx.sampled ? "01" : "00"}`;
}

/**
 * Parse a W3C `traceparent` header value. Strict: version `00` only,
 * all-zero ids rejected per the W3C spec. Returns `null` when invalid.
 */
export function fromTraceparent(header: unknown): TraceContext | null {
  if (typeof header !== "string") return null;
  const parts = header.trim().split("-");
  if (parts.length !== 4) return null;
  const [version, traceId, spanId, flags] = parts;
  if (version !== "00") return null;
  if (!isValidTraceId(traceId) || traceId === ZERO_TRACE_ID) return null;
  if (!isValidSpanId(spanId) || spanId === ZERO_SPAN_ID) return null;
  if (!TRACE_FLAGS_RE.test(flags)) return null;
  const sampled = (Number.parseInt(flags, 16) & 1) === 1;
  return {
    trace_id: traceId,
    span_id: spanId,
    parent_span_id: null,
    sampled,
  };
}

/**
 * Sampling keep-rule (§7): sampled-in traces are kept; error spans
 * bypass sampling and are always kept.
 */
export function shouldKeep(ctx: TraceContext, isError: boolean): boolean {
  if (isError) return true;
  return ctx.sampled;
}
