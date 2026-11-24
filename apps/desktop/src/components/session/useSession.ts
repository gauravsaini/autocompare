import { useState, useEffect, useCallback, useRef } from "react";
import type { CachedSession, SessionId } from "../../ipc/types";
import {
  sessionList,
  sessionLoad,
  sessionSave,
  type SessionDocument,
  type SessionEntry,
  type SessionTab,
} from "../../ipc/session";
import { invoke } from "../../ipc/client";
import {
  open as tauriOpen,
  save as tauriSave,
  type OpenDialogOptions,
  type SaveDialogOptions,
} from "@tauri-apps/plugin-dialog";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __MOCK_DIALOG_OPEN__?: (
      options?: OpenDialogOptions,
    ) => Promise<string | string[] | null>;
    __MOCK_DIALOG_SAVE__?: (
      options?: SaveDialogOptions,
    ) => Promise<string | null>;
  }
}

/**
 * Native file open dialog helper with mock fallbacks.
 * Never throws to callers; returns null if cancelled or unavailable.
 */
export async function pickOpenDialog(
  options?: OpenDialogOptions,
): Promise<string | string[] | null> {
  if (
    typeof window !== "undefined" &&
    typeof window.__MOCK_DIALOG_OPEN__ === "function"
  ) {
    try {
      return await window.__MOCK_DIALOG_OPEN__(options);
    } catch (err: unknown) {
      console.warn("Mock open dialog error:", err);
      return null;
    }
  }
  if (typeof window !== "undefined" && !window.__TAURI_INTERNALS__) {
    // In mock mode / browser without Tauri runtime, return null gracefully.
    return null;
  }
  try {
    return await tauriOpen(options);
  } catch (err: unknown) {
    console.warn("Dialog open error:", err);
    return null;
  }
}

/**
 * Native file save dialog helper with mock fallbacks.
 * Never throws to callers; returns null if cancelled or unavailable.
 */
export async function pickSaveDialog(
  options?: SaveDialogOptions,
): Promise<string | null> {
  if (
    typeof window !== "undefined" &&
    typeof window.__MOCK_DIALOG_SAVE__ === "function"
  ) {
    try {
      return await window.__MOCK_DIALOG_SAVE__(options);
    } catch (err: unknown) {
      console.warn("Mock save dialog error:", err);
      return null;
    }
  }
  if (typeof window !== "undefined" && !window.__TAURI_INTERNALS__) {
    return null;
  }
  try {
    return await tauriSave(options);
  } catch (err: unknown) {
    console.warn("Dialog save error:", err);
    return null;
  }
}

/**
 * Wave-5A tab/session dedup helpers.
 *
 * Root cause of the "N identical tabs" bug: `openSession` minted a fresh
 * `sess-mem-*` id on EVERY call, so repeating the same kind+paths (double
 * submit, re-open of the same report) stacked identical tabs instead of
 * focusing the existing session. These helpers give every open request a
 * stable identity signature so open/load paths dedup by id and
 * focus-existing instead of duplicating.
 */

/** localStorage key for last-active session restore across boots. */
const LAST_ACTIVE_KEY = "autocompare:lastActiveSessionId";

/**
 * Wave-6A: max persisted sessions restored into the list on boot/refresh.
 * The on-disk store is never trimmed (no deletions); anything beyond the
 * newest N rows simply stays unlisted until the pile shrinks.
 */
export const SESSION_RESTORE_LIMIT = 20;

function normalizePathForKey(p: string | null | undefined): string {
  return (p ?? "").replace(/\\/g, "/").trim();
}

function normalizeKindForKey(kind: string | null | undefined): string {
  return (kind ?? "").trim().toLowerCase();
}

/**
 * Wave-6A canonical identity key for "same session" comparison.
 * Unlike the Wave-5A concatenation, fields are newline-separated so
 * `("text"+"/a"+"/b")` can never collide with `("text/a"+""+"/b")`.
 * Returns null when the row carries no identity at all (vacuous doc:
 * blank kind AND blank paths) — null keys are dropped, never merged.
 *
 * Wave-134: trailing base (ancestor) segment. Merge sessions that differ
 * only in base are different sessions; non-merge rows pass null and keep
 * the legacy 3-segment prefix plus an empty tail, so old and new keys
 * never collide across versions (all keys are minted by this fn only).
 */
export function sessionRowKey(
  kind: string | null | undefined,
  left: string | null | undefined,
  right: string | null | undefined,
  base?: string | null | undefined,
): string | null {
  if (isEmptySessionIdentity(kind, left, right)) return null;
  return `${normalizeKindForKey(kind)}\n${normalizePathForKey(left)}\n${normalizePathForKey(right)}\n${normalizePathForKey(base)}`;
}

/** Wave-6A: true when kind AND all paths are blank (nothing to render). */
export function isEmptySessionIdentity(
  kind: string | null | undefined,
  left: string | null | undefined,
  right: string | null | undefined,
): boolean {
  return (
    normalizeKindForKey(kind) === "" &&
    normalizePathForKey(left) === "" &&
    normalizePathForKey(right) === ""
  );
}

/**
 * Wave-6A legacy-pile collapse (pure, no IPC).
 *
 * Groups rows by `keyOf` identity, keeps the newest id per group
 * (backend mints `sess-<millis>-<counter>`, so lexicographic max is the
 * most recently saved row), drops null-key (empty) rows, and caps the
 * result at the newest `limit` rows. Never deletes on disk — hidden
 * rows simply stay unlisted.
 */
export function collapseSessionRows<T>(
  rows: readonly T[],
  keyOf: (row: T) => string | null,
  limit: number = SESSION_RESTORE_LIMIT,
  idOf: (row: T) => string = (row) => (row as { readonly id: string }).id,
): {
  readonly rows: T[];
  readonly merged: number;
  readonly droppedEmpty: number;
  readonly trimmed: number;
} {
  const seen = new Map<string, T>();
  let droppedEmpty = 0;
  for (const row of rows) {
    if (!row) {
      droppedEmpty += 1;
      continue;
    }
    const id = idOf(row);
    if (typeof id !== "string" || id.trim() === "") {
      droppedEmpty += 1;
      continue;
    }
    let key: string | null = null;
    try {
      key = keyOf(row);
    } catch {
      key = `\0id:${id}`;
    }
    if (key === null) {
      droppedEmpty += 1;
      continue;
    }
    const prev = seen.get(key);
    if (!prev || id > idOf(prev)) seen.set(key, row);
  }
  const merged = rows.length - droppedEmpty - seen.size;
  const sorted = [...seen.values()].sort((a, b) => {
    const ai = idOf(a);
    const bi = idOf(b);
    return ai < bi ? 1 : ai > bi ? -1 : 0;
  });
  const trimmed = Math.max(0, sorted.length - limit);
  return {
    rows: sorted.slice(0, limit),
    merged,
    droppedEmpty,
    trimmed,
  };
}

function baseNameOf(p: string): string {
  const base = p.replace(/\\/g, "/").split("/").pop() || p;
  return base;
}

/** Display name for kind+paths (single source; App/SessionManager reuse it). */
export function sessionDisplayName(
  kind: string,
  left?: string | null,
  right?: string | null,
): string {
  const sessionKind = kind || "text";
  const leftBase = left ? baseNameOf(left) : null;
  const rightBase = right ? baseNameOf(right) : null;
  return leftBase && rightBase
    ? `${sessionKind}: ${leftBase} ↔ ${rightBase}`
    : `${sessionKind} session`;
}

/** First-wins dedup by session id, preserving order. */
export function dedupSessionsById<T extends { id: string }>(list: readonly T[]): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const item of list) {
    if (!item || seen.has(item.id)) continue;
    seen.add(item.id);
    out.push(item);
  }
  return out;
}

function readLastActiveId(): string | null {
  try {
    if (typeof window === "undefined" || !window.localStorage) return null;
    return window.localStorage.getItem(LAST_ACTIVE_KEY);
  } catch {
    return null;
  }
}

function writeLastActiveId(id: string | null): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    if (id === null) {
      window.localStorage.removeItem(LAST_ACTIVE_KEY);
    } else {
      window.localStorage.setItem(LAST_ACTIVE_KEY, id);
    }
  } catch {
    // Storage unavailable (private mode, tests): restore is best-effort.
  }
}

/**
 * Return signature of useSession hook.
 */
export interface UseSessionReturn {
  /** Available cached and in-memory sessions */
  readonly sessions: CachedSession[];
  /** Currently active session identifier, or null */
  readonly activeId: SessionId | null;
  /**
   * Opens or creates a new session.
   * Resolves the minted session id, or null on error.
   */
  readonly openSession: (
    kind: string,
    left?: string | null,
    right?: string | null,
    base?: string | null,
  ) => Promise<string | null>;
  /**
   * Closes a session by id.
   * Resolves true on success, false on error.
   */
  readonly closeSession: (id: string) => Promise<boolean>;
  /**
   * Saves a session descriptor via IPC.
   * `live` carries the caller's live tab state (App `tabPaths`); when
   * present it overwrites the active tab's kind/paths/options so save
   * persists what is on screen, never stale nulls. `path` (save dialog)
   * is honored when the backend supports it; the current `session_save`
   * mints an id and ignores the path, which is noted via console.warn.
   * Resolves the saved session id or path, or null on error.
   */
  readonly saveSession: (
    id?: string,
    path?: string,
    live?: {
      readonly kind?: string | null;
      readonly left?: string | null;
      readonly right?: string | null;
      readonly base?: string | null;
      readonly output?: string | null;
      readonly options?: Record<string, string> | null;
    } | null,
  ) => Promise<string | null>;
  /**
   * Loads a saved session from disk or session store.
   * Prompts dialog when path is omitted.
   * Resolves loaded session id, or null on error.
   */
  readonly loadSession: (path?: string) => Promise<string | null>;
  /** Refreshes session list from backend IPC. */
  readonly refresh: () => Promise<void>;
  /**
   * Wave-123: returns the cached full descriptor for a session id, or
   * null when not loaded yet. App uses this (or `sessionLoad` IPC) to
   * hydrate `tabPaths` from the real doc instead of blank hardcodes.
   */
  readonly getSessionDoc: (id: string) => SessionDocument | null;
  /**
   * Deletes a session by id.
   * Resolves true on success, false on error.
   */
  readonly deleteSession: (id: string) => Promise<boolean>;
  /** Activates a session by id */
  readonly selectSession: (id: string | null) => void;
  /** Active session object, or null */
  readonly activeSession: CachedSession | null;
  /**
   * Wave-6A: how many duplicate/empty rows the last restore or load
   * collapsed away (`merged + droppedEmpty`). SessionManager renders a
   * transient "Cleaned up N duplicate sessions" note from this.
   */
  readonly cleanupCount: number;
  /** Inline error message if any, never thrown to UI */
  readonly error: string | null;
  /** Resets inline error */
  readonly clearError: () => void;
}

/**
 * Session management hook (Wave 2C).
 *
 * Integrates `session_*` backend IPC (`session_list`, `session_load`, `session_save`,
 * and graceful `session_delete`) with `@tauri-apps/plugin-dialog` open/save dialogs.
 * Guaranteed: errors are surfaced as inline strings in `error` and NEVER thrown to UI.
 */
export function useSession(): UseSessionReturn {
  const [sessions, setSessions] = useState<CachedSession[]>([]);
  const [activeId, setActiveId] = useState<SessionId | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [cleanupCount, setCleanupCount] = useState(0);

  // In-memory cache for full session descriptors
  const docCacheRef = useRef<Record<string, SessionDocument>>({});

  // Wave-6A: source rows retired by saveSession. `session_save` mints a
  // brand-new id every call, so without this the just-saved in-memory
  // row and its persisted twin share one identity and the restore
  // collapse would hide the id saveSession just activated.
  const retiredIdsRef = useRef<Set<string>>(new Set());

  // Wave-129: tracked save path per session id (BC parity). A session
  // saved/opened with an explicit file path saves silently to that
  // path; untracked (new) sessions get the Save-As dialog instead.
  const savePathRef = useRef<Map<string, string>>(new Map());

  // Wave-5A: refs mirror state so open/close paths never act on stale
  // closures (the old closeSession read `sessions` from its render scope
  // and could activate a just-closed or duplicated id).
  const sessionsRef = useRef<CachedSession[]>([]);
  const activeIdRef = useRef<SessionId | null>(null);
  sessionsRef.current = sessions;
  activeIdRef.current = activeId;

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const refresh = useCallback(async (): Promise<void> => {
    try {
      const entries: SessionEntry[] = await sessionList();
      // Dedup backend rows by id (first wins): directory races can
      // surface the same id twice, and restore must never re-add one.
      // Newest-first: the backend sorts ascending by minted id, so the
      // last persisted row sorts first and becomes the focused default.
      const byId = dedupSessionsById(
        (entries || [])
          .filter((entry) => entry && typeof entry === "object" && entry.id)
          .map((entry) => ({
            id: entry.id,
            kind: "text",
            name: entry.name || entry.id,
            locked: Boolean(entry.locked),
            pageCount: Number(entry.tabs) || 1,
          })),
      );
      const newestFirst = [...byId].sort((a, b) =>
        a.id < b.id ? 1 : a.id > b.id ? -1 : 0,
      );
      const savedIds = new Set(newestFirst.map((s) => s.id));

      // Wave-6A legacy-pile cleanup: resolve every persisted row to its
      // real kind+paths via session_load (real IPC, best-effort), then
      // collapse identical identities into the newest id, drop vacuous
      // rows, and cap the restore window. Unloadable entries keep a
      // unique fallback key: unknown identity is preserved, never merged
      // and never dropped.
      interface MergeRow {
        readonly session: CachedSession;
        readonly key: string | null;
        readonly doc: SessionDocument | null;
        readonly persisted: boolean;
      }
      const mergeInput: MergeRow[] = [];
      for (const entry of newestFirst) {
        try {
          const doc = await sessionLoad(entry.id);
          const tabs = doc && Array.isArray(doc.tabs) ? doc.tabs : [];
          if (tabs.length === 0) {
            mergeInput.push({
              session: entry,
              key: null,
              doc: null,
              persisted: true,
            });
            continue;
          }
          // Wave-123: identity/kind from the doc's active tab (clamped),
          // not always tabs[0], so multi-tab docs restore focused.
          const activeIdx =
            typeof doc.active_tab_index === "number" &&
            doc.active_tab_index >= 0 &&
            doc.active_tab_index < tabs.length
              ? doc.active_tab_index
              : 0;
          const tab = tabs[activeIdx] ?? tabs[0];
          const sigKind =
            typeof tab.session_type === "string" ? tab.session_type : "";
          const left =
            typeof tab.left_path === "string" ? tab.left_path : null;
          const right =
            typeof tab.right_path === "string" ? tab.right_path : null;
          if (!(entry.id in docCacheRef.current)) {
            docCacheRef.current[entry.id] = doc;
          }
          mergeInput.push({
            session: {
              ...entry,
              kind: sigKind.trim() === "" ? "text" : sigKind,
              pageCount: tabs.length,
            },
            key: sessionRowKey(sigKind, left, right, tab.base_path ?? null),
            doc,
            persisted: true,
          });
        } catch {
          mergeInput.push({
            session: entry,
            key: `\0id:${entry.id}`,
            doc: null,
            persisted: true,
          });
        }
      }

      // Preserve listed sessions the backend snapshot has not seen yet
      // (memory rows, plus just-saved ids whose list snapshot predates
      // the save). Retired rows (already re-saved under a minted id)
      // stay out so the collapse below cannot resurrect them next to
      // their twin. Deleted rows never reach here: deleteSession drops
      // them from sessionsRef synchronously.
      for (const s of sessionsRef.current) {
        if (savedIds.has(s.id) || retiredIdsRef.current.has(s.id)) {
          continue;
        }
        const cached = docCacheRef.current[s.id];
        const tab = cached && cached.tabs.length > 0 ? cached.tabs[0] : null;
        mergeInput.push({
          session: s,
          key:
            tab !== null
              ? sessionRowKey(tab.session_type, tab.left_path, tab.right_path, tab.base_path ?? null)
              : `\0id:${s.id}`,
          doc: cached ?? null,
          persisted: false,
        });
      }

      const collapsed = collapseSessionRows(
        mergeInput,
        (row) => row.key,
        SESSION_RESTORE_LIMIT,
        (row) => row.session.id,
      );
      const keptIds = new Set(collapsed.rows.map((row) => row.session.id));
      // Prune descriptor cache entries for rows the collapse hid, so a
      // later focus-existing scan cannot match a tab that is no longer
      // listed. Docs for listed sessions are untouched.
      for (const row of mergeInput) {
        if (!keptIds.has(row.session.id)) {
          delete docCacheRef.current[row.session.id];
        }
      }
      const merged = collapsed.rows.map((row) => row.session);
      setSessions(merged);

      // Restore keeps last active: prefer the live active id, else the
      // persisted last-active id when it still exists, else the first row.
      const liveActive =
        activeIdRef.current !== null &&
        merged.some((s) => s.id === activeIdRef.current)
          ? activeIdRef.current
          : null;
      const stored = readLastActiveId();
      const restored =
        liveActive ??
        (stored !== null && merged.some((s) => s.id === stored) ? stored : null) ??
        (merged.length > 0 ? merged[0].id : null);
      setActiveId(restored);
      writeLastActiveId(restored);
      setCleanupCount(collapsed.merged + collapsed.droppedEmpty);
      setError(null);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(`Failed to refresh sessions: ${msg}`);
    }
  }, []);

  const openSession = useCallback(
    async (
      kind: string,
      left?: string | null,
      right?: string | null,
      base?: string | null,
    ): Promise<string | null> => {
      try {
        const sessionKind = kind || "text";
        const leftNorm = left ?? null;
        const rightNorm = right ?? null;
        const baseNorm = base ?? null;
        const wanted = sessionRowKey(sessionKind, leftNorm, rightNorm, baseNorm);

        // Focus-existing: an identical kind+paths(+base) session already
        // open must be activated, never duplicated into a second tab.
        const existing = sessionsRef.current.find((s) => {
          const doc = docCacheRef.current[s.id];
          if (doc && doc.tabs.length > 0) {
            const tab = doc.tabs[0];
            const have = sessionRowKey(
              tab.session_type,
              tab.left_path,
              tab.right_path,
              tab.base_path ?? null,
            );
            // A vacuous cached doc (null key) never matches by identity:
            // it carries nothing to focus. The display-name fallback below
            // still applies so nameless rows behave as before.
            if (wanted === null || have === null) return false;
            return have === wanted;
          }
          return (
            s.kind === sessionKind &&
            s.name === sessionDisplayName(sessionKind, leftNorm, rightNorm)
          );
        });
        if (existing) {
          setActiveId(existing.id);
          writeLastActiveId(existing.id);
          setError(null);
          return existing.id;
        }

        const memId = `sess-mem-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
        const name = sessionDisplayName(sessionKind, leftNorm, rightNorm);

        const doc: SessionDocument = {
          schema_version: 1,
          tabs: [
            {
              id: `tab-${memId}`,
              title: name,
              session_type: sessionKind,
              left_path: left ?? null,
              right_path: right ?? null,
              // Wave-134: base ancestor persists here (DTO field already
              // exists; no new IPC). If the backend rejects/ignores it,
              // the memory row below still carries it session-local.
              base_path: baseNorm,
              output_path: null,
              read_only: false,
              locked: false,
              options: {},
            },
          ],
          active_tab_index: 0,
          name,
          locked: false,
        };

        // Wave-65: auto-persist the new tab through the existing
        // session_save IPC (no dialog — the explicit save flow keeps
        // that) BEFORE listing it, so the listed id is the persisted
        // id from birth: no mem→saved swap, no retire, no orphan
        // cleanup, and App state (demoId, tabPaths) stays consistent.
        // A backend without a session store, or a failed save, falls
        // back to the memory row, so tabs never break when persistence
        // cannot.
        let id = memId;
        try {
          const saved = (await sessionSave(doc)) as unknown;
          if (typeof saved === "string" && saved !== "") {
            id = saved;
          }
        } catch {
          // Best-effort: the memory row below keeps working.
        }

        const newSession: CachedSession = {
          id,
          kind: sessionKind,
          name,
          locked: false,
          pageCount: 1,
        };

        docCacheRef.current[id] = doc;
        // Insert exactly once: guard against double-submit races by
        // filtering the id first, then prepending.
        setSessions((prev) =>
          dedupSessionsById([newSession, ...prev.filter((s) => s.id !== id)]),
        );
        setActiveId(id);
        writeLastActiveId(id);
        // Self-heal against a stale mount-refresh snapshot racing this
        // insert (its list predates the save): re-list now that the row
        // is persisted, then keep the just-opened tab focused.
        await refresh();
        setActiveId(id);
        writeLastActiveId(id);
        setError(null);
        return id;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(`Failed to open session: ${msg}`);
        return null;
      }
    },
    [],
  );

  const closeSession = useCallback(
    async (id: string): Promise<boolean> => {
      try {
        // Wave-65: the store mirrors open tabs (auto-persist on open),
        // so closing also drops the persisted row — otherwise the next
        // refresh would resurrect the closed tab. Best-effort and
        // silent: memory-only rows were never persisted, and some
        // backends do not register session_delete.
        try {
          await invoke("session_delete", { id });
        } catch {
          // Best-effort cleanup only.
        }
        delete docCacheRef.current[id];
        // Remove exactly one session by id. The next active tab is
        // computed from the post-removal list (never the stale render
        // scope): prefer the persisted last-active id when it survives,
        // else the first remaining session, else null.
        const remaining = dedupSessionsById(
          sessionsRef.current.filter((s) => s.id !== id),
        );
        setSessions(remaining);
        if (activeIdRef.current === id) {
          const stored = readLastActiveId();
          const next =
            stored !== null && remaining.some((s) => s.id === stored)
              ? stored
              : (remaining.length > 0 ? remaining[0].id : null);
          setActiveId(next);
          writeLastActiveId(next);
        } else {
          // Active tab untouched; still drop the stored id if it pointed
          // at the closed session.
          if (readLastActiveId() === id) {
            writeLastActiveId(activeIdRef.current);
          }
        }
        setError(null);
        return true;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(`Failed to close session: ${msg}`);
        return false;
      }
    },
    [],
  );

  const saveSession = useCallback(
    async (
      id?: string,
      path?: string,
      live?: {
        readonly kind?: string | null;
        readonly left?: string | null;
        readonly right?: string | null;
        readonly base?: string | null;
        readonly output?: string | null;
        readonly options?: Record<string, string> | null;
      } | null,
    ): Promise<string | null> => {
      try {
        const targetId = id ?? activeIdRef.current;
        if (!targetId) {
          setError("No session selected to save");
          return null;
        }

        const cached = sessionsRef.current.find((s) => s.id === targetId);
        const existingDoc = docCacheRef.current[targetId];

        // Wave-123: persist LIVE tab state, never stale nulls. When the
        // caller passes `live` (App tabPaths) it wins; otherwise fall back
        // to the cached doc's active tab so a bare toolbar Save keeps paths.
        const liveKind =
          live?.kind ?? existingDoc?.tabs[existingDoc.active_tab_index]?.session_type ?? cached?.kind ?? "text";
        const liveLeft =
          live?.left ?? existingDoc?.tabs[existingDoc.active_tab_index]?.left_path ?? null;
        const liveRight =
          live?.right ?? existingDoc?.tabs[existingDoc.active_tab_index]?.right_path ?? null;
        const liveBase =
          live?.base ?? existingDoc?.tabs[existingDoc.active_tab_index]?.base_path ?? null;
        const liveOutput =
          live?.output ?? existingDoc?.tabs[existingDoc.active_tab_index]?.output_path ?? null;
        const liveOptions =
          live?.options ??
          existingDoc?.tabs[existingDoc.active_tab_index]?.options ??
          {};

        let targetDoc: SessionDocument;
        if (existingDoc) {
          const idx =
            existingDoc.active_tab_index >= 0 &&
            existingDoc.active_tab_index < existingDoc.tabs.length
              ? existingDoc.active_tab_index
              : 0;
          const nextTabs = existingDoc.tabs.map((t, i) =>
            i === idx
              ? {
                  ...t,
                  session_type: liveKind || t.session_type || "text",
                  left_path: liveLeft,
                  right_path: liveRight,
                  base_path: liveBase,
                  output_path: liveOutput,
                  options: { ...(liveOptions as Record<string, string>) },
                }
              : t,
          );
          targetDoc = {
            ...existingDoc,
            tabs: nextTabs,
            name: cached?.name ?? existingDoc.name,
            locked: cached?.locked ?? existingDoc.locked,
          };
        } else {
          targetDoc = {
            schema_version: 1,
            tabs: [
              {
                id: `tab-${targetId}`,
                title: cached?.name || "Compare Session",
                session_type: liveKind || cached?.kind || "text",
                left_path: liveLeft,
                right_path: liveRight,
                base_path: liveBase,
                output_path: liveOutput,
                read_only: false,
                locked: cached?.locked || false,
                options: { ...(liveOptions as Record<string, string>) },
              },
            ],
            active_tab_index: 0,
            name: cached?.name || "Compare Session",
            locked: cached?.locked || false,
          };
        }

        let targetPath = path ?? savePathRef.current.get(targetId);
        if (targetPath === undefined) {
          const dialogResult = await pickSaveDialog({
            defaultPath: `${cached?.name || "session"}.json`,
            filters: [{ name: "AutoCompare Session", extensions: ["json"] }],
          });
          if (!dialogResult) {
            // Wave-129: Save-As cancelled — abort with no phantom save.
            return null;
          }
          targetPath = dialogResult;
        }
        if (targetPath) {
          // Honestly ignored: session_save mints an id and takes no path.
          // Note it instead of pretending the file landed at targetPath.
          console.warn(`session_save ignores dialog path, persisted under minted id: ${targetPath}`);
        }

        // Call sessionSave IPC command
        const savedId = await sessionSave(targetDoc);
        const resolvedId = savedId || targetId;

        // Wave-6A: the backend mints a fresh id per save, so the source
        // row (usually an in-memory `sess-mem-*` row) is retired here.
        // Otherwise the next refresh would list the source and its twin
        // under one identity and the collapse would hide the id we are
        // about to activate.
        if (resolvedId !== targetId) {
          retiredIdsRef.current.add(targetId);
          if (retiredIdsRef.current.size > 200) {
            retiredIdsRef.current.clear();
            retiredIdsRef.current.add(targetId);
          }
          delete docCacheRef.current[targetId];
          savePathRef.current.delete(targetId);
        }
        // Wave-129: the path (explicit, dialog-picked, or tracked) follows
        // the minted id so the next save stays silent.
        savePathRef.current.set(resolvedId, targetPath);

        // Persist doc in cache
        docCacheRef.current[resolvedId] = targetDoc;

        await refresh();
        setActiveId(resolvedId);
        writeLastActiveId(resolvedId);
        setError(null);
        return resolvedId;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(`Failed to save session: ${msg}`);
        return null;
      }
    },
    [refresh],
  );

  const loadSession = useCallback(
    async (path?: string): Promise<string | null> => {
      try {
        let loadPath = path;
        if (!loadPath) {
          const dialogResult = await pickOpenDialog({
            multiple: false,
            filters: [{ name: "AutoCompare Session", extensions: ["json"] }],
          });
          if (typeof dialogResult === "string") {
            loadPath = dialogResult;
          } else if (Array.isArray(dialogResult) && dialogResult.length > 0) {
            loadPath = dialogResult[0];
          }
        }

        if (!loadPath) {
          // Cancelled by user or dialog unavailable
          return null;
        }

        // Derive session id from path or input
        const sessionId = loadPath.endsWith(".json")
          ? loadPath.replace(/\\/g, "/").split("/").pop()?.replace(/\.json$/, "") || loadPath
          : loadPath;

        // Wave-123: no text-blank fallback for stored-id loads. A failed
        // id load is an inline error, never a synthesized fake doc that
        // hides data loss. Exception: an explicit file-path pick (dialog
        // returns a slash path) imports as a new session row so the
        // Open-dialog flow keeps working; stored ids stay honest.
        let rawDoc: SessionDocument;
        try {
          rawDoc = await sessionLoad(sessionId);
        } catch (err: unknown) {
          const looksLikeFilePath = /[/\\]/.test(loadPath);
          if (!looksLikeFilePath) {
            const msg = err instanceof Error ? err.message : String(err);
            setError(`Failed to load session: ${msg}`);
            return null;
          }
          rawDoc = {
            schema_version: 1,
            tabs: [
              {
                id: `tab-${sessionId}`,
                title: sessionId,
                session_type: "text",
                left_path: loadPath,
                right_path: null,
                base_path: null,
                output_path: null,
                read_only: false,
                locked: false,
                options: {},
              },
            ],
            active_tab_index: 0,
            name: sessionId,
            locked: false,
          };
        }
        if (!rawDoc || typeof rawDoc !== "object" || !Array.isArray((rawDoc as SessionDocument).tabs)) {
          setError("Failed to load session: Invalid descriptor shape");
          return null;
        }
        const doc: SessionDocument = rawDoc;

        docCacheRef.current[sessionId] = doc;
        const tabs: readonly SessionTab[] = doc.tabs ?? [];
        // Wave-123: hydrate from the doc's active tab (clamped), so kind +
        // left/right/base/output_path + options survive the round trip.
        const activeIdx =
          typeof doc.active_tab_index === "number" &&
          doc.active_tab_index >= 0 &&
          doc.active_tab_index < tabs.length
            ? doc.active_tab_index
            : 0;
        const activeTab = tabs[activeIdx] ?? tabs[0];
        const kind = activeTab?.session_type || "text";
        const name = doc.name || sessionId;

        const loadedSession: CachedSession = {
          id: sessionId,
          kind,
          name,
          locked: Boolean(doc.locked),
          pageCount: tabs.length || 1,
        };

        // Wave-6A: loading a session whose kind+paths already list must
        // not stack a second row. Drop only rows with a PROVEN equal
        // identity (doc-cached); unknown rows are always preserved.
        // Wave-123: compare against each row's active tab, not tabs[0].
        const loadedKey = sessionRowKey(
          activeTab?.session_type ?? null,
          activeTab?.left_path ?? null,
          activeTab?.right_path ?? null,
          activeTab?.base_path ?? null,
        );
        const prunedIds = new Set<string>();
        if (loadedKey !== null) {
          for (const s of sessionsRef.current) {
            if (s.id === sessionId) continue;
            const cached = docCacheRef.current[s.id];
            if (!cached || cached.tabs.length === 0) continue;
            const cIdx =
              typeof cached.active_tab_index === "number" &&
              cached.active_tab_index >= 0 &&
              cached.active_tab_index < cached.tabs.length
                ? cached.active_tab_index
                : 0;
            const tab = cached.tabs[cIdx] ?? cached.tabs[0] ?? null;
            if (tab === null || tab === undefined) continue;
            if (
              sessionRowKey(tab.session_type, tab.left_path, tab.right_path, tab.base_path ?? null) ===
              loadedKey
            ) {
              prunedIds.add(s.id);
            }
          }
        }
        for (const prunedId of prunedIds) {
          delete docCacheRef.current[prunedId];
        }
        if (prunedIds.size > 0) {
          setCleanupCount((c) => c + prunedIds.size);
        }

        setSessions((prev) => {
          const idx = prev.findIndex((s) => s.id === sessionId);
          const upserted =
            idx >= 0
              ? prev.map((s, i) => (i === idx ? loadedSession : s))
              : [loadedSession, ...prev];
          if (prunedIds.size === 0) return dedupSessionsById(upserted);
          return dedupSessionsById(
            upserted.filter(
              (s) => s.id === sessionId || !prunedIds.has(s.id),
            ),
          );
        });

        setActiveId(sessionId);
        writeLastActiveId(sessionId);
        setError(null);
        return sessionId;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(`Failed to load session: ${msg}`);
        return null;
      }
    },
    [],
  );

  const deleteSession = useCallback(
    async (id: string): Promise<boolean> => {
      try {
        try {
          await invoke("session_delete", { id });
        } catch (ipcErr: unknown) {
          // session_delete command may not be registered in backend yet;
          // recorded as gap for Wave 2R.
          console.warn("session_delete IPC not available or failed:", ipcErr);
        }

        delete docCacheRef.current[id];
        savePathRef.current.delete(id);
        // Remove exactly one session by id; when the deleted session was
        // active, fall through to the next remaining session instead of
        // dropping to null (keeps one tab focused).
        const remaining = dedupSessionsById(
          sessionsRef.current.filter((s) => s.id !== id),
        );
        setSessions(remaining);
        if (activeIdRef.current === id) {
          const next = remaining.length > 0 ? remaining[0].id : null;
          setActiveId(next);
          writeLastActiveId(next);
        } else if (readLastActiveId() === id) {
          writeLastActiveId(activeIdRef.current);
        }
        setError(null);
        return true;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(`Failed to delete session: ${msg}`);
        return false;
      }
    },
    [],
  );

  const selectSession = useCallback((id: string | null) => {
    setActiveId(id);
    // Persist only valid selections; a null selection keeps the stored
    // id so a later refresh can still restore it when present.
    if (id !== null) {
      writeLastActiveId(id);
    }
  }, []);

  const getSessionDoc = useCallback((id: string): SessionDocument | null => {
    return docCacheRef.current[id] ?? null;
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const activeSession = sessions.find((s) => s.id === activeId) ?? null;

  return {
    sessions,
    activeId,
    openSession,
    closeSession,
    saveSession,
    loadSession,
    refresh,
    getSessionDoc,
    deleteSession,
    selectSession,
    activeSession,
    cleanupCount,
    error,
    clearError,
  };
}
