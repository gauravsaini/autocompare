import { invoke as tauriInvoke } from "@tauri-apps/api/core";

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
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (cfg.token) {
    headers["X-AutoCompare-Token"] = cfg.token;
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
  if (rpcCfg !== null && import.meta.env.VITE_MOCK_IPC !== "1") {
    return httpRpcInvoke<T>(rpcCfg, cmd, args);
  }
  if (isMockIpc()) {
    const mod = await import("./mockAdapter");
    return mod.mockInvoke<T>(cmd, args);
  }
  return tauriInvoke<T>(cmd, args);
}

/** Test-only helper forwarded to the lazy mock adapter. */
export async function __mockTextEditResetForTests(): Promise<void> {
  const mod = await import("./mockAdapter");
  mod.__mockTextEditResetForTests();
}
