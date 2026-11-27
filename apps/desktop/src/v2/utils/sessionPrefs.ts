import type { SessionKind } from "../types";

const PINNED_KEY = "autocompare:v2:pinnedSessions";
const RESTORE_ENABLED_KEY = "autocompare:v2:restoreLastSession";
const LAST_SESSION_KEY = "autocompare:v2:lastSession";
const FILE_WATCH_MODE_KEY = "autocompare:v2:fileWatchMode";

export type FileWatchMode = "prompt" | "auto" | "off";

export interface LastSessionSnapshot {
  readonly kind: Exclude<SessionKind, "home">;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly title: string;
}

export function loadPinnedSessionIds(): string[] {
  try {
    if (typeof window === "undefined" || !window.localStorage) return [];
    const raw = window.localStorage.getItem(PINNED_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

export function savePinnedSessionIds(ids: readonly string[]): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(PINNED_KEY, JSON.stringify(Array.from(new Set(ids))));
  } catch {
    // Ignore storage errors
  }
}

export function togglePinnedId(current: readonly string[], id: string): string[] {
  return current.includes(id) ? current.filter((x) => x !== id) : [id, ...current];
}

export function sortSessionsWithPins<T extends { readonly id: string }>(
  items: readonly T[],
  pinnedIds: readonly string[],
): T[] {
  const pinnedSet = new Set(pinnedIds);
  const pinned: T[] = [];
  const rest: T[] = [];
  for (const item of items) {
    if (pinnedSet.has(item.id)) pinned.push(item);
    else rest.push(item);
  }
  return [...pinned, ...rest];
}

export function isRestoreLastSessionEnabled(): boolean {
  try {
    if (typeof window === "undefined" || !window.localStorage) return false;
    return window.localStorage.getItem(RESTORE_ENABLED_KEY) === "1";
  } catch {
    return false;
  }
}

export function setRestoreLastSessionEnabled(enabled: boolean): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(RESTORE_ENABLED_KEY, enabled ? "1" : "0");
  } catch {
    // Ignore storage errors
  }
}

export function saveLastSessionSnapshot(snapshot: LastSessionSnapshot): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(LAST_SESSION_KEY, JSON.stringify(snapshot));
  } catch {
    // Ignore storage errors
  }
}

export function loadLastSessionSnapshot(): LastSessionSnapshot | null {
  try {
    if (typeof window === "undefined" || !window.localStorage) return null;
    const raw = window.localStorage.getItem(LAST_SESSION_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<LastSessionSnapshot>;
    if (!parsed || typeof parsed.kind !== "string" || typeof parsed.title !== "string") {
      return null;
    }
    return {
      kind: parsed.kind as Exclude<SessionKind, "home">,
      leftPath: typeof parsed.leftPath === "string" ? parsed.leftPath : "",
      rightPath: typeof parsed.rightPath === "string" ? parsed.rightPath : "",
      title: parsed.title,
    };
  } catch {
    return null;
  }
}

export function normalizeFileWatchMode(raw: unknown): FileWatchMode {
  if (raw === "auto" || raw === "off" || raw === "prompt") return raw;
  return "prompt";
}

export function loadFileWatchMode(): FileWatchMode {
  try {
    if (typeof window === "undefined" || !window.localStorage) return "prompt";
    return normalizeFileWatchMode(window.localStorage.getItem(FILE_WATCH_MODE_KEY));
  } catch {
    return "prompt";
  }
}

export function saveFileWatchMode(mode: FileWatchMode): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(FILE_WATCH_MODE_KEY, mode);
    window.dispatchEvent(new CustomEvent("autocompare:filewatch-changed", { detail: mode }));
  } catch {
    // Ignore storage errors
  }
}
