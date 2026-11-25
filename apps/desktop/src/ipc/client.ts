import { invoke as tauriInvoke } from "@tauri-apps/api/core";

/**
 * Production-thin Tauri `invoke` wrapper.
 *
 * - Production desktop shell: calls `@tauri-apps/api/core` `invoke` directly.
 * - Mock / browser preview (`VITE_MOCK_IPC=1` or outside Tauri runtime):
 *   lazy-loads `./mockAdapter` on demand so production bundles stay lean.
 */
function isMockIpc(): boolean {
  if (import.meta.env.VITE_MOCK_IPC === "1") return true;
  if (typeof window !== "undefined") {
    const w = window as unknown as Record<string, unknown>;
    if (w["__TAURI_INTERNALS__"] === undefined && w["__TAURI__"] === undefined) {
      return true;
    }
  }
  return false;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
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
