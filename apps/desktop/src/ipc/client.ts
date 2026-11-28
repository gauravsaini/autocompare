import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  parse,
  toTraceparent,
  type TraceContext,
} from "../observability/context";
import {
  endSpan,
  isTelemetryEnabled,
  startRootSpan,
} from "../observability/tracer";
import {
  listRecentSpans,
  type SpanQuery,
} from "../observability/query";

/**
 * Unified frontend IPC transport.
 *
 * Transport resolution order:
 * 1. HTTP JSON-RPC (`window.__AUTOCOMPARE_RPC__` or `VITE_RPC_URL` for `autocompare web`)
 * 2. Mock / browser preview (`VITE_MOCK_IPC=1` or running outside Tauri without RPC config)
 * 3. Native Tauri desktop IPC (`@tauri-apps/api/core` `invoke`)
 */
export interface HttpRpcConfig {
  url: string;
  token?: string;
}

interface RpcEnvelope<T> {
  ok: boolean;
  result?: T;
  error?: string;
}

let originRpcBootstrapPromise: Promise<HttpRpcConfig | null> | null = null;

/**
 * Discovers `/rpc-config.json` when served by `autocompare web --dist ...`.
 * Caches the lookup so mock/Tauri environments incur at most one fast check.
 */
export function bootstrapHttpRpcFromOrigin(): Promise<HttpRpcConfig | null> {
  if (originRpcBootstrapPromise) return originRpcBootstrapPromise;
  if (typeof window === "undefined" || typeof fetch !== "function") {
    originRpcBootstrapPromise = Promise.resolve(null);
    return originRpcBootstrapPromise;
  }
  const w = window as unknown as { __AUTOCOMPARE_RPC__?: HttpRpcConfig };
  if (w.__AUTOCOMPARE_RPC__ && typeof w.__AUTOCOMPARE_RPC__.url === "string") {
    originRpcBootstrapPromise = Promise.resolve(w.__AUTOCOMPARE_RPC__);
    return originRpcBootstrapPromise;
  }
  originRpcBootstrapPromise = fetch("/rpc-config.json", { cache: "no-store" })
    .then(async (resp) => {
      if (!resp.ok) return null;
      const cfg = (await resp.json()) as Partial<HttpRpcConfig>;
      if (typeof cfg.url === "string" && cfg.url.length > 0) {
        const resolved: HttpRpcConfig = { url: cfg.url, token: cfg.token };
        w.__AUTOCOMPARE_RPC__ = resolved;
        return resolved;
      }
      return null;
    })
    .catch(() => null);
  return originRpcBootstrapPromise;
}

function getHttpRpcConfig(): HttpRpcConfig | null {
  if (typeof window !== "undefined") {
    const w = window as unknown as { __AUTOCOMPARE_RPC__?: HttpRpcConfig };
    if (w.__AUTOCOMPARE_RPC__ && typeof w.__AUTOCOMPARE_RPC__.url === "string") {
      return w.__AUTOCOMPARE_RPC__;
    }
  }
  const envUrl = import.meta.env.VITE_RPC_URL as string | undefined;
  if (envUrl && envUrl.trim().length > 0) {
    const envToken = import.meta.env.VITE_RPC_TOKEN as string | undefined;
    return { url: envUrl.trim(), token: envToken };
  }
  return null;
}

function isMockIpc(): boolean {
  if (import.meta.env.VITE_MOCK_IPC === "1") return true;
  if (getHttpRpcConfig() !== null) return false;
  if (typeof window !== "undefined") {
    const w = window as unknown as Record<string, unknown>;
    if (w["__TAURI_INTERNALS__"] === undefined && w["__TAURI__"] === undefined) {
      return true;
    }
  }
  return false;
}

async function httpRpcInvoke<T>(
  cfg: HttpRpcConfig,
  method: string,
  params?: Record<string, unknown>,
  traceparent?: string,
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (cfg.token) {
    headers["X-AutoCompare-Token"] = cfg.token;
  }
  if (traceparent !== undefined) {
    headers["traceparent"] = traceparent;
  }
  const resp = await fetch(cfg.url, {
    method: "POST",
    headers,
    body: JSON.stringify({ method, params: params ?? {} }),
  });
  if (!resp.ok) {
    throw new Error(`HTTP RPC ${method} failed (${resp.status} ${resp.statusText})`);
  }
  const envelope = (await resp.json()) as RpcEnvelope<T>;
  if (!envelope.ok) {
    throw new Error(envelope.error ?? `RPC error for ${method}`);
  }
  return envelope.result as T;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const rpcCfg = getHttpRpcConfig();
  const mockForced = import.meta.env.VITE_MOCK_IPC === "1";
  // ODD Phase 3 vertical slice: trace propagation lives ONLY here (single
  // choke-point). When telemetry is off or no view started a root span,
  // fall through to the legacy paths byte-identically (zero behavior change).
  const ambient: TraceContext | null = isTelemetryEnabled() ? activeTrace : null;
  if (ambient === null) {
    if (rpcCfg !== null && !mockForced) {
      return httpRpcInvoke<T>(rpcCfg, cmd, args);
    }
    if (isMockIpc()) {
      const mod = await import("./mockAdapter");
      return mod.mockInvoke<T>(cmd, args);
    }
    return tauriInvoke<T>(cmd, args);
  }
  // Instrumented path: one child span per round-trip, same trace_id as the
  // calling (ambient) span. The span_id injected as __trace is this span's
  // id, so the responder's echo parent-links to it.
  const ipcSpan = startRootSpan({
    op: opForCommand(cmd),
    component: "ts.ipc/client",
    parent: ambient,
  });
  const params: Record<string, unknown> = { ...(args ?? {}), __trace: ipcSpan.context() };
  const startedAt = Date.now();
  let transport: string = "tauri";
  try {
    let result: T;
    if (rpcCfg !== null && !mockForced) {
      transport = "http";
      const traceparent = toTraceparent({
        trace_id: ipcSpan.trace_id,
        span_id: ipcSpan.span_id,
        parent_span_id: ipcSpan.parent_span_id,
        sampled: ipcSpan.sampled,
      });
      result = await httpRpcInvoke<T>(rpcCfg, cmd, params, traceparent);
    } else if (isMockIpc()) {
      transport = "mock";
      const mod = await import("./mockAdapter");
      result = await mod.mockInvoke<T>(cmd, params);
    } else {
      transport = "tauri";
      result = await tauriInvoke<T>(cmd, params);
    }
    const attrs: Record<string, unknown> = {
      transport,
      duration_ms: Math.max(0, Date.now() - startedAt),
    };
    // Response correlation check (§5.2): warn-shaped attribute when the
    // responder echo is missing or belongs to another trace (catches
    // mock/real divergence without touching the result).
    let echoOk = false;
    let sawMock = false;
    if (result !== null && typeof result === "object" && !Array.isArray(result)) {
      const envelope = result as Record<string, unknown>;
      if (envelope["__mock"] === true) sawMock = true;
      const echoed = parse(envelope["__trace"]);
      echoOk = echoed !== null && echoed.trace_id === ipcSpan.trace_id;
    }
    attrs["trace_echo"] = echoOk;
    if (sawMock) attrs["is_mock"] = true;
    ipcSpan.set(attrs);
    endSpan(ipcSpan);
    return result;
  } catch (err) {
    ipcSpan.set({ transport });
    ipcSpan.error({
      kind: "invoke_failed",
      message: err instanceof Error ? err.message : String(err),
    });
    endSpan(ipcSpan);
    throw err;
  }
}

/** Test-only helper forwarded to the lazy mock adapter. */
export async function __mockTextEditResetForTests(): Promise<void> {
  const mod = await import("./mockAdapter");
  mod.__mockTextEditResetForTests();
}

// ---------------------------------------------------------------------------
// ODD Phase 3: ambient trace context + op mapping + e2e introspection hook.
// ---------------------------------------------------------------------------

/** Trace context of the currently-executing user operation (set by views). */
let activeTrace: TraceContext | null = null;

/**
 * Set the ambient trace that the next `invoke` calls join. Accepts a
 * `TraceContext` or a `__trace` envelope; malformed input clears to null.
 * Views call this right after `startRootSpan` and `clearActiveTrace` in
 * `finally`, so at most one user operation owns the ambient slot.
 */
export function setActiveTrace(ctx: unknown): void {
  try {
    activeTrace = parse(ctx);
  } catch {
    activeTrace = null;
  }
}

/** Release the ambient slot (no-op when already empty). */
export function clearActiveTrace(): void {
  activeTrace = null;
}

/** Text-domain commands share the `text.compare` op (one user operation). */
const TEXT_OP_COMMANDS: ReadonlySet<string> = new Set([
  "text_compare",
  "text_get_rows",
  "text_get_overview",
  "text_find",
  "text_copy_section",
  "text_save_side",
  "text_apply_edit",
  "text_apply_hunk",
  "text_restore_rows",
  "text_update_side",
  "stat_paths",
]);

/** Map an IPC command to its telemetry op (schema `op` is free-form). */
function opForCommand(cmd: string): string {
  if (TEXT_OP_COMMANDS.has(cmd)) return "text.compare";
  const sep = cmd.indexOf("_");
  if (sep > 0) return `${cmd.slice(0, sep)}.${cmd.slice(sep + 1)}`;
  return cmd;
}

// Read-only e2e introspection (mock-mode trace assertions). Registered once
// at module init; it only reads the in-memory sink buffer and never alters
// IPC behavior, so it is inert when telemetry is disabled (buffer stays empty).
try {
  // Dev/mock only: the introspection hook must never ship in production
  // shells (buffer stays empty there anyway, but internals stay hidden).
  const exposeHook =
    typeof window !== "undefined" &&
    (import.meta.env.DEV || import.meta.env.VITE_MOCK_IPC === "1");
  if (exposeHook) {
    const w = window as unknown as Record<string, unknown>;
    if (w["__AUTOCOMPARE_TRACE__"] === undefined) {
      w["__AUTOCOMPARE_TRACE__"] = {
        listRecentSpans: (query?: SpanQuery) => listRecentSpans(query),
      };
    }
  }
} catch {
  // Introspection must never break the IPC path.
}
