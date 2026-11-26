import { useCallback, useEffect, useRef, useState } from "react";
import { statPaths } from "../../ipc/text";
import type { TextFileMeta } from "../../ipc/text";
import { detectChangedPaths } from "../utils/fileWatch";

export interface UseExternalChangeWatchOptions {
  readonly paths: readonly string[];
  readonly enabled?: boolean;
  readonly intervalMs?: number;
}

export interface ExternalChangeWatchState {
  readonly changedPaths: readonly string[];
  readonly dismiss: () => void;
  readonly resetBaseline: () => Promise<void>;
}

/**
 * Watches underlying comparison files for external disk modifications
 * (checks on window focus, tab visibility restore, and a gentle poll interval).
 * Call `resetBaseline()` after intentional reloads or in-app saves to avoid
 * self-triggered prompts.
 */
export function useExternalChangeWatch({
  paths,
  enabled = true,
  intervalMs = 3000,
}: UseExternalChangeWatchOptions): ExternalChangeWatchState {
  const [changedPaths, setChangedPaths] = useState<readonly string[]>([]);
  const baselineRef = useRef<readonly (TextFileMeta | null)[] | null>(null);
  const pathsKey = paths.join("\0");
  const pathsRef = useRef(paths);
  pathsRef.current = paths;

  const fetchStats = useCallback(async (targetPaths: readonly string[]) => {
    const clean = targetPaths.filter((p) => p.trim().length > 0);
    if (clean.length === 0) return null;
    try {
      const res = await statPaths(targetPaths);
      return Array.isArray(res) ? res : null;
    } catch {
      return null;
    }
  }, []);

  const resetBaseline = useCallback(async (): Promise<void> => {
    setChangedPaths([]);
    const stats = await fetchStats(pathsRef.current);
    baselineRef.current = stats;
  }, [fetchStats]);

  const dismiss = useCallback((): void => {
    void resetBaseline();
  }, [resetBaseline]);

  useEffect(() => {
    if (!enabled) {
      setChangedPaths([]);
      baselineRef.current = null;
      return;
    }

    let alive = true;
    baselineRef.current = null;
    setChangedPaths([]);

    const init = async (): Promise<void> => {
      const stats = await fetchStats(pathsRef.current);
      if (alive) baselineRef.current = stats;
    };
    void init();

    const check = async (): Promise<void> => {
      if (!alive) return;
      const current = await fetchStats(pathsRef.current);
      if (!alive || !current) return;
      if (baselineRef.current === null) {
        baselineRef.current = current;
        return;
      }
      const changed = detectChangedPaths(pathsRef.current, baselineRef.current, current);
      if (changed.length > 0) {
        setChangedPaths(changed);
      }
    };

    const onFocus = (): void => {
      void check();
    };
    const onVisibility = (): void => {
      if (document.visibilityState === "visible") void check();
    };

    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisibility);
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") void check();
    }, intervalMs);

    return () => {
      alive = false;
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibility);
      window.clearInterval(timer);
    };
  }, [pathsKey, enabled, intervalMs, fetchStats]);

  return { changedPaths, dismiss, resetBaseline };
}
