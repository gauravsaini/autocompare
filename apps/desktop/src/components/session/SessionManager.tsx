import React, { useState, useEffect, useRef } from "react";
import {
  X,
  FolderOpen,
  Save,
  Trash2,
  Plus,
  FileText,
  Folder,
  Binary,
  Table,
  Image as ImageIcon,
  GitMerge,
  RefreshCw,
  FolderSearch,
  Pencil,
  Copy,
  Star,
  Check,
} from "lucide-react";
import type { CachedSession } from "../../ipc/types";
import { sessionLoad, sessionSave, settingsGet, settingsSet } from "../../ipc/session";
import { useSession, pickOpenDialog, dedupSessionsById } from "./useSession";
import { textCompareDemo, TEXT_DEMO_CANDIDATES } from "../../ipc/text";
import { extractDroppedPath } from "../dnd/dropPaths";

export interface SessionManagerProps {
  /** Controls dialog open/close visibility. Defaults to true if omitted. */
  readonly isOpen?: boolean;
  /** Invoked when user requests dialog to close. */
  readonly onClose?: () => void;
  /** Controlled active session ID. Overrides internal hook state when provided. */
  readonly activeId?: string | null;
  /** Controlled sessions list. Overrides internal hook state when provided. */
  readonly sessions?: readonly CachedSession[];
  /** Callback fired whenever active session or selection changes. */
  readonly onChange?: (activeSession: CachedSession | null) => void;
  /** Custom handler for opening/creating a new session. */
  readonly onOpenSession?: (
    kind: string,
    left?: string | null,
    right?: string | null,
    base?: string | null,
  ) => Promise<string | null> | void;
  /** Custom handler for closing a session. */
  readonly onCloseSession?: (id: string) => Promise<boolean> | void;
  /** Custom handler for saving a session. */
  readonly onSaveSession?: (id?: string, path?: string) => Promise<string | null> | void;
  /** Custom handler for loading a session. */
  readonly onLoadSession?: (path?: string) => Promise<string | null> | void;
  /** Custom handler for deleting a session. */
  readonly onDeleteSession?: (id: string) => Promise<boolean> | void;
  /**
   * Wave-6A: restore-collapse count owned by the App-level `useSession`
   * (`merged + droppedEmpty`). When omitted, the dialog falls back to its
   * internal hook's count. A positive value renders a transient
   * "Cleaned up N duplicate sessions" note (`session/cleanup-note`).
   */
  readonly cleanupCount?: number | null;
  /** Optional custom CSS classes for dialog container. */
  readonly className?: string;
}

const KIND_OPTIONS = [
  { value: "text", label: "Text Compare", icon: FileText },
  { value: "folder", label: "Folder Compare", icon: Folder },
  { value: "hex", label: "Hex Compare", icon: Binary },
  { value: "table", label: "Table Compare", icon: Table },
  { value: "media", label: "Media Compare", icon: ImageIcon },
  { value: "merge", label: "Merge 3-Way", icon: GitMerge },
  { value: "sync", label: "Folder Sync", icon: RefreshCw },
];

/**
 * SessionManager dialog component (Wave 2C).
 *
 * Props-driven dialog providing:
 * 1. Recent sessions list (`data-testid="session/list"`)
 * 2. Toolbar action buttons: Open (`session/open`), Save (`session/save`), Delete (`session/delete`)
 * 3. New session form (`data-testid="session/new"`) with kind selector and dual path pickers
 * 4. Inline error display and `onChange` callback for Wave-2R mounting
 */
export function SessionManager({
  isOpen = true,
  onClose,
  activeId: controlledActiveId,
  sessions: controlledSessions,
  onChange,
  onOpenSession,
  onSaveSession,
  onLoadSession,
  onDeleteSession,
  cleanupCount: cleanupCountProp,
  className = "",
}: SessionManagerProps): React.JSX.Element | null {
  const hook = useSession();

  const [sessions, setSessions] = useState<readonly CachedSession[]>(
    controlledSessions ?? hook.sessions,
  );
  const [activeId, setActiveId] = useState<string | null>(
    controlledActiveId !== undefined ? controlledActiveId : hook.activeId,
  );

  // Wave-6A: transient restore-collapse note. The count comes from the
  // App-level hook when controlled, else the internal hook. Re-arm on
  // every count change and on dialog open; auto-dismiss after 6s with
  // plain setTimeout (no new deps). Rendered only while positive.
  const effectiveCleanupCount = cleanupCountProp ?? hook.cleanupCount;
  const [cleanupVisible, setCleanupVisible] = useState(true);
  useEffect(() => {
    setCleanupVisible(true);
  }, [effectiveCleanupCount]);
  useEffect(() => {
    if (isOpen) setCleanupVisible(true);
  }, [isOpen]);
  useEffect(() => {
    if (effectiveCleanupCount <= 0 || !cleanupVisible) return;
    const timer = setTimeout(() => setCleanupVisible(false), 6000);
    return () => clearTimeout(timer);
  }, [effectiveCleanupCount, cleanupVisible]);

  // New session form state
  const [kind, setKind] = useState<string>("text");
  const [leftPath, setLeftPath] = useState<string>("");
  const [rightPath, setRightPath] = useState<string>("");
  // Wave-134: merge ancestor (base) input. Visible for merge kind only;
  // other kinds render an honest collapsed note instead.
  const [basePath, setBasePath] = useState<string>("");

  // Wave-117: input drop affordance (BC parity — file drops onto the
  // left/right/base target inputs stage that side via the shared
  // `extractDroppedPath` helper). Outline-only, never layout: the inputs
  // keep their size, only `outline` + a `data-drop-active` hook change.
  const [inputDrop, setInputDrop] = useState<"left" | "right" | "base" | null>(null);

  const inputDropHandlers = (side: "left" | "right" | "base") => ({
    onDragOver: (e: React.DragEvent<HTMLInputElement>) => {
      e.preventDefault();
      try {
        if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      } catch {
        // Best-effort only.
      }
      setInputDrop(side);
    },
    onDragLeave: (e: React.DragEvent<HTMLInputElement>) => {
      if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
      setInputDrop((prev) => (prev === side ? null : prev));
    },
    onDrop: (e: React.DragEvent<HTMLInputElement>) => {
      e.preventDefault();
      e.stopPropagation();
      setInputDrop(null);
      const path = extractDroppedPath(e.dataTransfer);
      if (!path) return;
      if (side === "left") setLeftPath(path);
      else if (side === "right") setRightPath(path);
      else setBasePath(path);
    },
  });

  const inputDropStyle = (side: "left" | "right" | "base"): React.CSSProperties | undefined =>
    inputDrop === side
      ? { outline: "2px dashed var(--ac-accent)", outlineOffset: "-1px" }
      : undefined;

  // Wave-25 row actions (Beyond Compare style): rename inline, clone with
  // copy suffix, two-step delete with last-row refusal, favorite star.
  // All persistence goes through the existing session/settings IPC
  // (`sessionSave`, `settingsGet`/`settingsSet`, `hook.deleteSession`
  // which wraps `session_delete`); every failure renders inline and
  // never throws to the dialog.
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [draftName, setDraftName] = useState<string>("");
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  // Wave-129: toolbar delete arms a two-step confirm like row delete
  // (BC parity) and never deletes the last row.
  const [confirmToolbarDelete, setConfirmToolbarDelete] = useState(false);
  const [favorites, setFavorites] = useState<ReadonlySet<string>>(new Set());
  const [rowError, setRowError] = useState<string | null>(null);
  const [rowErrorId, setRowErrorId] = useState<string | null>(null);

  // Wave-40 session picker filter (Beyond Compare style): live
  // case-insensitive substring across name/kind/paths. Filter-only state:
  // `sessions` stays the source of truth, selection (`activeId`) is never
  // touched by the filter, and all wave-25 row actions operate on the same
  // row objects (filtered rows render the untouched row JSX).
  const [search, setSearch] = useState<string>("");

  const FAVORITES_STORAGE_KEY = "autocompare:sessionFavorites";

  function readFavoritesFromSettingsValue(value: unknown): string[] {
    try {
      if (value === null || value === undefined) return [];
      if (typeof value === "string") {
        return value
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s !== "");
      }
      const obj = value as { readonly extra?: unknown };
      if (obj && typeof obj === "object" && "extra" in obj) {
        const extra = (obj as { readonly extra?: unknown }).extra;
        if (extra && typeof extra === "object") {
          const raw = (extra as Record<string, unknown>)["sessionFavorites"];
          if (typeof raw === "string") return readFavoritesFromSettingsValue(raw);
        }
      }
    } catch {
      // Fall through to empty: favorites are best-effort, never a crash.
    }
    return [];
  }

  // Load persisted favorites once (existing settings IPC; mock IPC
  // resolves `[]` which simply yields no favorites). Falls back to
  // localStorage so the star survives reloads even when settings IPC
  // is unavailable.
  useEffect(() => {
    let cancelled = false;
    try {
      const stored = window.localStorage.getItem(FAVORITES_STORAGE_KEY);
      if (stored && !cancelled) {
        const ids = stored
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s !== "");
        if (ids.length > 0) setFavorites(new Set(ids));
      }
    } catch {
      // Storage unavailable: ignore, settings IPC is still attempted.
    }
    void (async () => {
      try {
        const settings = await settingsGet();
        if (cancelled) return;
        const ids = readFavoritesFromSettingsValue(settings);
        if (ids.length > 0) {
          setFavorites(new Set(ids));
        }
      } catch {
        // Best-effort only: keep localStorage state, never crash.
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function persistFavorites(next: ReadonlySet<string>): Promise<void> {
    try {
      window.localStorage.setItem(FAVORITES_STORAGE_KEY, [...next].join(","));
    } catch {
      // Private mode etc: settings IPC below is still attempted.
    }
    try {
      await settingsSet({ extra: { sessionFavorites: [...next].join(",") } });
    } catch (err: unknown) {
      // Mock IPC and locked-settings backends reject unknown flows;
      // the star already toggled locally, so surface inline only.
      const msg = err instanceof Error ? err.message : String(err);
      setRowError(`Could not persist favorite: ${msg}`);
      setRowErrorId(null);
    }
  }

  async function persistSessionDoc(
    id: string,
    name: string,
    kindValue: string,
    locked: boolean,
  ): Promise<void> {
    // Wave-123: rename/clone must preserve paths/options. Load the stored
    // doc first and only rename; never synthesize null paths over live data.
    try {
      const existing = await sessionLoad(id).catch(() => null);
      if (existing && Array.isArray(existing.tabs) && existing.tabs.length > 0) {
        await sessionSave({
          ...existing,
          tabs: existing.tabs.map((t, i) =>
            i === (existing.active_tab_index ?? 0)
              ? { ...t, title: name, session_type: kindValue || t.session_type, locked }
              : t,
          ),
          name,
          locked,
        });
        return;
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setRowError(`Could not persist session: ${msg}`);
      setRowErrorId(id);
      return;
    }
    // Best-effort persistence of rename/clone rows through the existing
    // `session_save` IPC. The local list is already updated optimistically
    // by the caller; a backend refusal only produces an inline note.
    try {
      await sessionSave({
        schema_version: 1,
        tabs: [
          {
            id: `tab-${id}`,
            title: name,
            session_type: kindValue,
            left_path: null,
            right_path: null,
            base_path: null,
            output_path: null,
            read_only: false,
            locked,
            options: {},
          },
        ],
        active_tab_index: 0,
        name,
        locked,
      });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setRowError(`Could not persist session: ${msg}`);
      setRowErrorId(id);
    }
  }

  const handleStartRename = (session: CachedSession) => {
    if (session.locked) {
      setRowError("Locked sessions cannot be renamed.");
      setRowErrorId(session.id);
      return;
    }
    setRenamingId(session.id);
    setDraftName(session.name);
    setRowError(null);
    setRowErrorId(null);
    setConfirmDeleteId(null);
  };

  const handleCancelRename = () => {
    setRenamingId(null);
    setDraftName("");
  };

  const handleSaveRename = (session: CachedSession) => {
    const next = draftName.trim();
    if (next === "") {
      setRowError("Name must not be empty.");
      setRowErrorId(session.id);
      return;
    }
    const renamed: CachedSession = { ...session, name: next };
    setSessions((prev) => prev.map((s) => (s.id === session.id ? renamed : s)));
    if (activeId === session.id) {
      onChange?.(renamed);
    }
    setRenamingId(null);
    setDraftName("");
    setRowError(null);
    setRowErrorId(null);
    void persistSessionDoc(session.id, next, session.kind, session.locked);
  };

  const handleClone = (session: CachedSession) => {
    const copyId = `${session.id}-copy-${Date.now().toString(36)}`;
    const copyName = `${session.name} (copy)`;
    const copy: CachedSession = {
      ...session,
      id: copyId,
      name: copyName,
      locked: false,
    };
    setSessions((prev) => {
      const idx = prev.findIndex((s) => s.id === session.id);
      if (idx < 0) return dedupSessionsById([...prev, copy]);
      const next = [...prev];
      next.splice(idx + 1, 0, copy);
      return dedupSessionsById(next);
    });
    setRowError(null);
    setRowErrorId(null);
    setConfirmDeleteId(null);
    // Wave-123: clone preserves the source tabs/paths/options; only the
    // name/lock change. Best-effort: load source doc, save under copy name.
    void (async () => {
      try {
        const src = await sessionLoad(session.id).catch(() => null);
        if (src && Array.isArray(src.tabs)) {
          await sessionSave({
            ...src,
            tabs: src.tabs.map((t) => ({ ...t, locked: false })),
            name: copyName,
            locked: false,
          });
          return;
        }
      } catch {
        // Fall through to placeholder persist below.
      }
      void persistSessionDoc(copyId, copyName, session.kind, false);
    })();
  };

  const handleRowDeleteRequest = (session: CachedSession) => {
    // At least one session always remains: refusing the last row keeps
    // the workspace bootable and mirrors Beyond Compare's guard.
    if (sessions.length <= 1) {
      setRowError("At least one session must remain.");
      setRowErrorId(session.id);
      return;
    }
    setRowError(null);
    setRowErrorId(null);
    setConfirmDeleteId(session.id);
  };

  const handleRowDeleteConfirm = async (session: CachedSession) => {
    if (sessions.length <= 1) {
      setRowError("At least one session must remain.");
      setRowErrorId(session.id);
      setConfirmDeleteId(null);
      return;
    }
    try {
      let ok = true;
      if (onDeleteSession) {
        ok = (await onDeleteSession(session.id)) !== false;
      } else {
        ok = await hook.deleteSession(session.id);
      }
      if (!ok) {
        setRowError("Could not delete session.");
        setRowErrorId(session.id);
        setConfirmDeleteId(null);
        return;
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setRowError(`Could not delete session: ${msg}`);
      setRowErrorId(session.id);
      setConfirmDeleteId(null);
      return;
    }
    setSessions((prev) => prev.filter((s) => s.id !== session.id));
    setFavorites((prev) => {
      if (!prev.has(session.id)) return prev;
      const next = new Set(prev);
      next.delete(session.id);
      void persistFavorites(next);
      return next;
    });
    if (activeId === session.id) {
      const remaining = sessions.filter((s) => s.id !== session.id);
      const nextActive = remaining.length > 0 ? remaining[0] : null;
      setActiveId(nextActive ? nextActive.id : null);
      onChange?.(nextActive);
    }
    setConfirmDeleteId(null);
    setRowError(null);
    setRowErrorId(null);
    if (renamingId === session.id) {
      setRenamingId(null);
      setDraftName("");
    }
  };

  const handleToggleFavorite = (session: CachedSession) => {
    const next = new Set(favorites);
    if (next.has(session.id)) {
      next.delete(session.id);
    } else {
      next.add(session.id);
    }
    setFavorites(next);
    setRowError(null);
    setRowErrorId(null);
    void persistFavorites(next);
  };

  // Wave-25: signature-guarded sync. The pre-existing effect below
  // overwrote local rows on EVERY hook.sessions identity change, which
  // clobbered optimistic row actions (rename/clone) in controlled mode
  // whenever the internal hook churned (e.g. `hook.deleteSession` after
  // a row confirm). Now the list only resyncs when the incoming content
  // actually differs; genuine prop/hook updates still flow through.
  const lastSyncedSigRef = useRef<string>("");

  useEffect(() => {
    const incoming = controlledSessions !== undefined ? controlledSessions : hook.sessions;
    let sig = "";
    try {
      sig = incoming
        .map((s) => `${s.id}\n${s.name}\n${s.kind}\n${s.locked}\n${s.pageCount}`)
        .join("\n|\n");
    } catch {
      return;
    }
    if (sig === lastSyncedSigRef.current) return;
    lastSyncedSigRef.current = sig;
    if (controlledSessions !== undefined) {
      // Dedup by id on every sync: a duplicated store row must never
      // render as two list rows / two tabs.
      setSessions(dedupSessionsById(controlledSessions));
    } else {
      setSessions(dedupSessionsById(hook.sessions));
    }
  }, [controlledSessions, hook.sessions, lastSyncedSigRef]);

  useEffect(() => {
    if (controlledActiveId !== undefined) {
      setActiveId(controlledActiveId);
    } else {
      setActiveId(hook.activeId);
    }
  }, [controlledActiveId, hook.activeId]);

  // Wave-129: disarm the toolbar delete confirm whenever selection moves
  // so an armed confirm never fires against a different session.
  useEffect(() => {
    setConfirmToolbarDelete(false);
  }, [activeId]);

  if (!isOpen) {
    return null;
  }

  const effectiveActiveSession =
    sessions.find((s) => s.id === activeId) ?? null;

  // Wave-40: live filter. Matches name, kind, id, and any extra string /
  // string-array fields present at runtime (left/right/base/output paths
  // when a backend or future DTO carries them). `CachedSession` itself only
  // locks id/kind/name/locked/pageCount, so path text today arrives embedded
  // in `name` (e.g. "text: a.txt ↔ b.txt"); the generic scan keeps this
  // honest for both shapes without touching the contract.
  const searchQuery = search.trim().toLowerCase();
  const filteredSessions =
    searchQuery === ""
      ? sessions
      : sessions.filter((s) => {
          const haystacks: string[] = [s.name, s.kind, s.id];
          try {
            const record = s as unknown as Record<string, unknown>;
            for (const value of Object.values(record)) {
              if (typeof value === "string") {
                haystacks.push(value);
              } else if (
                Array.isArray(value) &&
                value.every((v) => typeof v === "string")
              ) {
                haystacks.push(...(value as string[]));
              }
            }
          } catch {
            // Fall through to name/kind/id only.
          }
          return haystacks.some((h) => h.toLowerCase().includes(searchQuery));
        });

  const handleClearSearch = () => {
    setSearch("");
  };

  const handleSelect = (session: CachedSession) => {
    setActiveId(session.id);
    hook.selectSession(session.id);
    onChange?.(session);
  };

  const handleOpenClick = async () => {
    let loadedId: string | null = null;
    if (onLoadSession) {
      const res = await onLoadSession();
      loadedId = typeof res === "string" ? res : null;
    } else {
      loadedId = await hook.loadSession();
    }
    if (loadedId) {
      setActiveId(loadedId);
      const target: CachedSession =
        sessions.find((s) => s.id === loadedId) ?? {
          id: loadedId,
          kind: "text",
          name: loadedId,
          locked: false,
          pageCount: 1,
        };
      // Upsert by id: focusing an existing session never adds a row.
      setSessions((prev) => dedupSessionsById([target, ...prev.filter((s) => s.id !== loadedId)]));
      onChange?.(target);
    }
  };

  const handleSaveClick = async () => {
    if (!activeId) return;
    let savedId: string | null = null;
    if (onSaveSession) {
      const res = await onSaveSession(activeId);
      savedId = typeof res === "string" ? res : null;
    } else {
      savedId = await hook.saveSession(activeId);
    }
    // Wave-129: follow the minted id so the dialog list/selection never
    // points at the retired pre-save row (post-save refresh, no stale).
    if (savedId !== null && savedId !== activeId) {
      const current = sessions.find((s) => s.id === activeId);
      const row: CachedSession = current
        ? { ...current, id: savedId }
        : { id: savedId, kind: "text", name: savedId, locked: false, pageCount: 1 };
      setSessions((prev) =>
        dedupSessionsById([row, ...prev.filter((s) => s.id !== savedId)]),
      );
      setActiveId(savedId);
      onChange?.(row);
    }
  };

  const handleDeleteClick = async () => {
    if (!activeId) return;
    // Wave-129: last row never deletes (mirrors the row guard).
    if (sessions.length <= 1) {
      setConfirmToolbarDelete(false);
      setRowError("At least one session must remain.");
      setRowErrorId(activeId);
      return;
    }
    // Wave-129: first click arms the confirm, second click deletes.
    if (!confirmToolbarDelete) {
      setRowError(null);
      setRowErrorId(null);
      setConfirmToolbarDelete(true);
      return;
    }
    setConfirmToolbarDelete(false);
    const targetId = activeId;
    if (onDeleteSession) {
      await onDeleteSession(targetId);
    } else {
      await hook.deleteSession(targetId);
    }
    // Wave-123: advance selection like the hook (first remaining), never
    // clobber to null while rows remain.
    const remaining = sessions.filter((s) => s.id !== targetId);
    setSessions(remaining);
    const next = remaining.length > 0 ? remaining[0]! : null;
    setActiveId(next ? next.id : null);
    onChange?.(next ?? null);
  };

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    // Wave-134: base ancestor rides along for merge kind only; other
    // kinds pass null (no fabricated base).
    const baseTrim = basePath.trim();
    const baseArg = kind === "merge" ? baseTrim || null : null;
    let newId: string | null = null;
    if (onOpenSession) {
      const res = await onOpenSession(
        kind,
        leftPath.trim() || null,
        rightPath.trim() || null,
        baseArg,
      );
      newId = typeof res === "string" ? res : null;
    } else {
      newId = await hook.openSession(
        kind,
        leftPath.trim() || null,
        rightPath.trim() || null,
        baseArg,
      );
    }

    if (newId) {
      setActiveId(newId);
      const activeObj: CachedSession = {
        id: newId,
        kind,
        name:
          leftPath && rightPath
            ? `${kind}: ${leftPath.split(/[/\\]/).pop()} ↔ ${rightPath.split(/[/\\]/).pop()}`
            : `${kind} session`,
        locked: false,
        pageCount: 1,
      };
      // Upsert by id: re-creating identical kind+paths focuses the
      // existing session instead of stacking a duplicate row/tab.
      setSessions((prev) => dedupSessionsById([activeObj, ...prev.filter((s) => s.id !== newId)]));
      onChange?.(activeObj);
    }
    setLeftPath("");
    setRightPath("");
    setBasePath("");
  };

  const handlePickLeft = async () => {
    const isDir = kind === "folder" || kind === "sync";
    const picked = await pickOpenDialog({
      directory: isDir,
      multiple: false,
      title: `Select Left ${isDir ? "Directory" : "File"}`,
    });
    if (typeof picked === "string") {
      setLeftPath(picked);
    } else if (Array.isArray(picked) && picked.length > 0) {
      setLeftPath(picked[0]);
    }
  };

  const handlePickRight = async () => {
    const isDir = kind === "folder" || kind === "sync";
    const picked = await pickOpenDialog({
      directory: isDir,
      multiple: false,
      title: `Select Right ${isDir ? "Directory" : "File"}`,
    });
    if (typeof picked === "string") {
      setRightPath(picked);
    } else if (Array.isArray(picked) && picked.length > 0) {
      setRightPath(picked[0]);
    }
  };

  // Wave-134: base ancestor picker (merge kind only; files, never dirs).
  const handlePickBase = async () => {
    const picked = await pickOpenDialog({
      directory: false,
      multiple: false,
      title: "Select Base File",
    });
    if (typeof picked === "string") {
      setBasePath(picked);
    } else if (Array.isArray(picked) && picked.length > 0) {
      setBasePath(picked[0]);
    }
  };

  // Wave-10 demo entry row: one-click bundled demo text pair via the real
  // text_compare IPC (same candidate-pair path TextView uses). Upserts by
  // id so re-clicking focuses instead of stacking a duplicate row/tab.
  const handleDemoClick = async () => {
    let left = TEXT_DEMO_CANDIDATES[0][0];
    let right = TEXT_DEMO_CANDIDATES[0][1];
    try {
      const found = await textCompareDemo();
      if (found !== null) {
        left = found.left;
        right = found.right;
      }
    } catch {
      // Fall back to the first candidate pair below.
    }
    let newId: string | null = null;
    if (onOpenSession) {
      const res = await onOpenSession("text", left, right);
      newId = typeof res === "string" ? res : null;
    } else {
      newId = await hook.openSession("text", left, right);
    }
    if (newId) {
      setActiveId(newId);
      const activeObj: CachedSession = {
        id: newId,
        kind: "text",
        name: `text: ${left.split(/[/\\]/).pop()} ↔ ${right.split(/[/\\]/).pop()}`,
        locked: false,
        pageCount: 1,
      };
      setSessions((prev) => dedupSessionsById([activeObj, ...prev.filter((s) => s.id !== newId)]));
      onChange?.(activeObj);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="session-manager-title"
      data-testid="session/dialog"
      className={`fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs ${className}`}
    >
      <div className="flex max-h-[90vh] w-full max-w-3xl flex-col rounded-xl border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] text-[var(--ac-fg)] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--ac-border)] px-6 py-4">
          <div className="flex items-center gap-3">
            <h2 id="session-manager-title" className="ac-title font-semibold text-[var(--ac-fg)]">
              Session Manager
            </h2>
            <span data-testid="session/count" className="rounded-full bg-[var(--ac-badge-bg)] px-2.5 py-0.5 ac-chrome-label font-medium text-[var(--ac-badge-fg)]">
              {searchQuery === ""
                ? `${sessions.length} ${sessions.length === 1 ? "session" : "sessions"}`
                : `${filteredSessions.length} of ${sessions.length} ${sessions.length === 1 ? "session" : "sessions"}`}
            </span>
          </div>

          <div className="flex items-center gap-2">
            {onClose && (
              <button
                type="button"
                onClick={onClose}
                aria-label="Close dialog"
                className="rounded-lg p-1.5 text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
              >
                <X size={18} />
              </button>
            )}
          </div>
        </div>

        {/* Toolbar Action Bar */}
        <div className="flex items-center justify-between border-b border-[var(--ac-border)] bg-[var(--ac-surface-secondary)] px-6 py-2.5">
          <div className="flex items-center gap-2">
            <button
              type="button"
              data-testid="session/open"
              onClick={handleOpenClick}
              className="flex items-center gap-1.5 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-1.5 ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              <FolderOpen size={14} />
              <span>Open</span>
            </button>

            <button
              type="button"
              data-testid="session/save"
              onClick={handleSaveClick}
              disabled={!activeId}
              className="flex items-center gap-1.5 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-1.5 ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-40"
            >
              <Save size={14} />
              <span>Save</span>
            </button>

            <button
              type="button"
              data-testid="session/delete"
              onClick={handleDeleteClick}
              disabled={!activeId}
              className="flex items-center gap-1.5 rounded-lg border border-red-500/30 bg-[var(--ac-bg)] px-3 py-1.5 ac-chrome-label font-medium text-red-600 dark:text-red-400 shadow-xs transition hover:bg-red-500/10 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-red-400 disabled:cursor-not-allowed disabled:opacity-40"
            >
              <Trash2 size={14} />
              <span>{confirmToolbarDelete ? "Confirm?" : "Delete"}</span>
            </button>
            {confirmToolbarDelete && (
              <>
                <button
                  type="button"
                  data-testid="session/delete-confirm"
                  onClick={handleDeleteClick}
                  className="flex items-center gap-1.5 rounded-lg bg-red-600 px-3 py-1.5 ac-chrome-label font-medium text-white shadow-xs transition hover:bg-red-700 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-red-400"
                >
                  <span>Confirm</span>
                </button>
                <button
                  type="button"
                  data-testid="session/delete-cancel"
                  onClick={() => setConfirmToolbarDelete(false)}
                  className="flex items-center gap-1.5 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-1.5 ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                >
                  <span>Cancel</span>
                </button>
              </>
            )}
          </div>

          {effectiveActiveSession && (
            <div className="ac-chrome-label text-[var(--ac-fg-muted)]">
              Active: <span className="font-medium text-[var(--ac-fg)]">{effectiveActiveSession.name}</span>
            </div>
          )}
        </div>

        {/* Wave-6A restore-collapse note (transient, auto-dismisses) */}
        {effectiveCleanupCount > 0 && cleanupVisible && (
          <div
            role="status"
            data-testid="session/cleanup-note"
            className="flex items-center justify-between border-b border-[var(--ac-border)] bg-[var(--ac-surface-secondary)] px-6 py-2 ac-chrome-label font-medium text-[var(--ac-fg-muted)]"
          >
            <span>
              Cleaned up {effectiveCleanupCount} duplicate{" "}
              {effectiveCleanupCount === 1 ? "session" : "sessions"}.
            </span>
            <button
              type="button"
              onClick={() => setCleanupVisible(false)}
              aria-label="Dismiss cleanup note"
              className="rounded p-0.5 transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              <X size={14} />
            </button>
          </div>
        )}

        {/* Inline Error Notice */}
        {hook.error && (
          <div
            role="alert"
            className="flex items-center justify-between border-b border-[var(--ac-alert-border)] bg-[var(--ac-alert-bg)] px-6 py-2 ac-chrome-label font-medium text-[var(--ac-alert-fg)]"
          >
            <span>{hook.error}</span>
            <button
              type="button"
              onClick={hook.clearError}
              className="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-200 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              <X size={14} />
            </button>
          </div>
        )}

        {/* Main Body Grid */}
        <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-2">
          {/* Left Column: Recent Sessions List */}
          <div className="flex flex-col border-b border-[var(--ac-border)] md:border-b-0 md:border-r">
            <div className="border-b border-[var(--ac-border)] px-6 py-2.5 text-xs font-semibold uppercase tracking-wider text-[var(--ac-fg-muted)]">
              Recent Sessions
            </div>

            {/* Wave-40 filter: narrows the list only, never the data. */}
            <div className="border-b border-[var(--ac-border)] px-4 py-2">
              <label htmlFor="session-search-input" className="sr-only">
                Filter sessions
              </label>
              <div className="relative">
                <input
                  id="session-search-input"
                  data-testid="session/search"
                  type="text"
                  role="searchbox"
                  placeholder="Filter by name, kind, or path…"
                  aria-label="Filter sessions by name, kind, or path"
                  autoComplete="off"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") {
                      e.stopPropagation();
                      handleClearSearch();
                    }
                  }}
                  className="w-full rounded-lg border border-[var(--ac-border)] bg-[var(--ac-pathbar-bg)] py-1.5 pl-3 pr-8 text-xs text-[var(--ac-fg)] placeholder:text-[var(--ac-fg-muted)] focus-visible:outline-hidden focus-visible:border-[var(--ac-accent)] focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                />
                {search !== "" && (
                  <button
                    type="button"
                    data-testid="session/search-clear"
                    onClick={handleClearSearch}
                    aria-label="Clear filter"
                    className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded p-1 text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                  >
                    <X size={13} />
                  </button>
                )}
              </div>
              {searchQuery !== "" && (
                <p data-testid="session/search-status" role="status" className="px-1 pt-1 text-[10px] text-[var(--ac-fg-muted)]">
                  {filteredSessions.length} of {sessions.length} shown
                </p>
              )}
            </div>

            <div
              data-testid="session/list"
              className="flex-1 space-y-1.5 overflow-y-auto p-4 max-h-[320px]"
            >
              {/* Wave-10 demo entry row: one click opens the bundled demo pair. */}
              <button
                type="button"
                data-testid="session/demo"
                onClick={() => void handleDemoClick()}
                className="group flex w-full cursor-pointer items-center justify-between rounded-lg border border-dashed border-[var(--ac-accent)]/60 bg-[var(--ac-surface)] px-3 py-2.5 text-left transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
              >
                <div className="flex items-center gap-3 overflow-hidden">
                  <FileText size={16} className="shrink-0 text-[var(--ac-accent)]" />
                  <div className="truncate">
                    <p className="truncate ac-chrome-label font-medium text-[var(--ac-fg)]">
                      Try the demo pair
                    </p>
                    <p className="text-[10px] text-[var(--ac-fg-muted)]">
                      text · bundled fixture, real compare
                    </p>
                  </div>
                </div>
              </button>
              {sessions.length === 0 ? (
                <div className="py-12 text-center ac-chrome-label text-[var(--ac-fg-muted)]">
                  No sessions found. Create a new session on the right.
                </div>
              ) : filteredSessions.length === 0 ? (
                <div
                  data-testid="session/search-empty"
                  className="py-12 text-center ac-chrome-label text-[var(--ac-fg-muted)]"
                >
                  <p>No sessions match “{search.trim()}”.</p>
                  <button
                    type="button"
                    data-testid="session/search-clear-empty"
                    onClick={handleClearSearch}
                    className="mt-2 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-1.5 ac-chrome-label font-medium text-[var(--ac-fg)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                  >
                    Clear filter
                  </button>
                </div>
              ) : (
                filteredSessions.map((session) => {
                  const isSelected = session.id === activeId;
                  const IconComp =
                    KIND_OPTIONS.find((k) => k.value === session.kind)?.icon ?? FileText;
                  const isRenaming = renamingId === session.id;
                  const isConfirmingDelete = confirmDeleteId === session.id;
                  const isFavorite = favorites.has(session.id);

                  return (
                    <div
                      key={session.id}
                      data-testid={`session/item-${session.id}`}
                      onClick={() => {
                        if (isRenaming || isConfirmingDelete) return;
                        handleSelect(session);
                      }}
                      className={`group flex cursor-pointer items-center justify-between gap-2 rounded-lg border px-3 py-2.5 transition ${
                        isSelected
                          ? "border-[var(--ac-accent)] bg-[var(--ac-selection-bg)] text-[var(--ac-selection-fg)] shadow-xs ring-1 ring-[var(--ac-accent)]"
                          : "border-[var(--ac-border)]/60 bg-[var(--ac-surface)] hover:border-[var(--ac-border)] hover:bg-[var(--ac-toolbar-btn-hover-bg)] text-[var(--ac-fg)]"
                      }`}
                    >
                      <div className="flex min-w-0 flex-1 items-center gap-3 overflow-hidden">
                        <IconComp size={16} className="shrink-0 text-[var(--ac-accent)]" />
                        <div className="min-w-0 flex-1 truncate">
                          {isRenaming ? (
                            <span
                              className="flex items-center gap-1"
                              onClick={(e) => e.stopPropagation()}
                            >
                              <input
                                data-testid={`session/rename-input-${session.id}`}
                                type="text"
                                value={draftName}
                                autoFocus
                                onChange={(e) => setDraftName(e.target.value)}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") handleSaveRename(session);
                                  if (e.key === "Escape") handleCancelRename();
                                }}
                                aria-label="Session name"
                                className="min-w-0 flex-1 rounded border border-[var(--ac-accent)] bg-[var(--ac-bg)] px-1.5 py-0.5 text-xs text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                              />
                              <button
                                type="button"
                                data-testid={`session/rename-save-${session.id}`}
                                onClick={() => handleSaveRename(session)}
                                aria-label="Save name"
                                className="rounded p-1 text-[var(--ac-accent)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                              >
                                <Check size={13} />
                              </button>
                              <button
                                type="button"
                                data-testid={`session/rename-cancel-${session.id}`}
                                onClick={handleCancelRename}
                                aria-label="Cancel rename"
                                className="rounded p-1 text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                              >
                                <X size={13} />
                              </button>
                            </span>
                          ) : (
                            <p className="truncate ac-chrome-label font-medium text-inherit">
                              {isFavorite ? (<Star size={11} aria-hidden="true" className="mr-1 inline text-amber-500" />) : null}{session.name}
                            </p>
                          )}
                          <p className="text-[10px] text-[var(--ac-fg-muted)]">
                            {session.kind} • {session.pageCount} {session.pageCount === 1 ? "tab" : "tabs"}
                          </p>
                        </div>
                      </div>

                      {session.locked && !isRenaming && (
                        <span className="rounded bg-amber-500/20 px-1.5 py-0.5 text-[9px] font-semibold text-amber-700 dark:text-amber-300">
                          Locked
                        </span>
                      )}

                      {/* Wave-25 row actions: favorite, rename, clone, delete. */}
                      {!isRenaming && !isConfirmingDelete && (
                        <div
                          className="flex shrink-0 items-center gap-0.5"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <button
                            type="button"
                            data-testid={`session/favorite-${session.id}`}
                            onClick={() => handleToggleFavorite(session)}
                            aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
                            aria-pressed={isFavorite}
                            title={isFavorite ? "Favorited" : "Favorite"}
                            className={`rounded p-1 transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)] ${
                              isFavorite
                                ? "text-amber-500"
                                : "text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
                            }`}
                          >
                            <Star size={13} fill={isFavorite ? "currentColor" : "none"} />
                          </button>
                          <button
                            type="button"
                            data-testid={`session/rename-${session.id}`}
                            onClick={() => handleStartRename(session)}
                            aria-label={`Rename ${session.name}`}
                            title="Rename"
                            className="rounded p-1 text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                          >
                            <Pencil size={13} />
                          </button>
                          <button
                            type="button"
                            data-testid={`session/clone-${session.id}`}
                            onClick={() => handleClone(session)}
                            aria-label={`Clone ${session.name}`}
                            title="Clone"
                            className="rounded p-1 text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                          >
                            <Copy size={13} />
                          </button>
                          <button
                            type="button"
                            data-testid={`session/row-delete-${session.id}`}
                            onClick={() => handleRowDeleteRequest(session)}
                            aria-label={`Delete ${session.name}`}
                            title="Delete"
                            className="rounded p-1 text-[var(--ac-fg-muted)] transition hover:bg-red-500/10 hover:text-red-600 dark:hover:text-red-400 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-red-400"
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                      )}

                      {/* Wave-25 inline delete confirm: keeps the last row. */}
                      {isConfirmingDelete && (
                        <div
                          className="flex shrink-0 items-center gap-1"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <button
                            type="button"
                            data-testid={`session/delete-confirm-${session.id}`}
                            onClick={() => void handleRowDeleteConfirm(session)}
                            className="rounded bg-red-600 px-2 py-1 text-[10px] font-medium text-white transition hover:bg-red-700 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-red-400"
                          >
                            Confirm
                          </button>
                          <button
                            type="button"
                            data-testid={`session/delete-cancel-${session.id}`}
                            onClick={() => setConfirmDeleteId(null)}
                            className="rounded border border-[var(--ac-border)] px-2 py-1 text-[10px] font-medium text-[var(--ac-fg)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                          >
                            Cancel
                          </button>
                        </div>
                      )}
                    </div>
                  );
                })
              )}
            </div>
            {/* Wave-25 inline row-action error (never a crash). */}
            {rowError && (
              <div
                role="alert"
                data-testid="session/row-error"
                data-row-id={rowErrorId ?? ""}
                className="mx-4 mb-3 rounded-lg border border-[var(--ac-alert-border)] bg-[var(--ac-alert-bg)] px-3 py-1.5 ac-meta font-medium text-[var(--ac-alert-fg)]"
              >
                {rowError}
              </div>
            )}
          </div>

          {/* Right Column: New Session Form */}
          <div className="flex flex-col">
            <div className="border-b border-[var(--ac-border)] px-6 py-2.5 text-xs font-semibold uppercase tracking-wider text-[var(--ac-fg-muted)]">
              New Session
            </div>

            <form
              data-testid="session/new"
              onSubmit={handleCreateSubmit}
              className="flex flex-1 flex-col justify-between space-y-4 p-6"
            >
              <div className="space-y-4">
                {/* Kind Selector */}
                <div>
                  <label
                    htmlFor="session-kind-select"
                    className="mb-1 block ac-chrome-label font-medium text-[var(--ac-fg-muted)]"
                  >
                    Session Type
                  </label>
                  <select
                    id="session-kind-select"
                    data-testid="session/kind-select"
                    value={kind}
                    onChange={(e) => setKind(e.target.value)}
                    className="w-full rounded-lg border border-[var(--ac-border)] bg-[var(--ac-pathbar-bg)] px-3 py-2 text-xs text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:border-[var(--ac-accent)] focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                  >
                    {KIND_OPTIONS.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Left Path Picker */}
                <div>
                  <label
                    htmlFor="session-left-path"
                    className="mb-1 block ac-chrome-label font-medium text-[var(--ac-fg-muted)]"
                  >
                    Left Target
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="session-left-path"
                      data-testid="session/left-path"
                      type="text"
                      placeholder="File or folder path..."
                      value={leftPath}
                      onChange={(e) => setLeftPath(e.target.value)}
                      data-drop-active={inputDrop === "left" ? "true" : "false"}
                      style={inputDropStyle("left")}
                      {...inputDropHandlers("left")}
                      className="flex-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-pathbar-bg)] px-3 py-2 text-xs text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:border-[var(--ac-accent)] focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                    />
                    <button
                      type="button"
                      onClick={handlePickLeft}
                      className="flex items-center gap-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-2 ac-chrome-label font-medium text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                    >
                      <FolderSearch size={14} />
                      <span>Browse</span>
                    </button>
                  </div>
                </div>

                {/* Right Path Picker */}
                <div>
                  <label
                    htmlFor="session-right-path"
                    className="mb-1 block ac-chrome-label font-medium text-[var(--ac-fg-muted)]"
                  >
                    Right Target
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="session-right-path"
                      data-testid="session/right-path"
                      type="text"
                      placeholder="File or folder path..."
                      value={rightPath}
                      onChange={(e) => setRightPath(e.target.value)}
                      data-drop-active={inputDrop === "right" ? "true" : "false"}
                      style={inputDropStyle("right")}
                      {...inputDropHandlers("right")}
                      className="flex-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-pathbar-bg)] px-3 py-2 text-xs text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:border-[var(--ac-accent)] focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                    />
                    <button
                      type="button"
                      onClick={handlePickRight}
                      className="flex items-center gap-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-2 ac-chrome-label font-medium text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                    >
                      <FolderSearch size={14} />
                      <span>Browse</span>
                    </button>
                  </div>
                </div>

                {/* Wave-134: Base ancestor (merge kind only). Other kinds
                    collapse to an honest note — no fabricated base. */}
                {kind === "merge" ? (
                  <div>
                    <label
                      htmlFor="session-base-path"
                      className="mb-1 block ac-chrome-label font-medium text-[var(--ac-fg-muted)]"
                    >
                      Base (Ancestor)
                    </label>
                    <div className="flex gap-2">
                      <input
                        id="session-base-path"
                        data-testid="session/base-path"
                        type="text"
                        placeholder="Ancestor file path (optional)..."
                        value={basePath}
                        onChange={(e) => setBasePath(e.target.value)}
                        data-drop-active={inputDrop === "base" ? "true" : "false"}
                        style={inputDropStyle("base")}
                        {...inputDropHandlers("base")}
                        className="flex-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-pathbar-bg)] px-3 py-2 text-xs text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:border-[var(--ac-accent)] focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                      />
                      <button
                        type="button"
                        onClick={handlePickBase}
                        className="flex items-center gap-1 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg)] px-3 py-2 ac-chrome-label font-medium text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                      >
                        <FolderSearch size={14} />
                        <span>Browse</span>
                      </button>
                    </div>
                  </div>
                ) : (
                  <p data-testid="session/base-note" className="text-[11px] text-[var(--ac-fg-muted)]">
                    Base ancestor applies to Merge 3-Way only.
                  </p>
                )}
              </div>

              {/* Submit Button */}
              <div className="pt-2">
                <button
                  type="submit"
                  data-testid="session/create-button"
                  className="flex w-full items-center justify-center gap-2 rounded-lg bg-[var(--ac-accent)] px-4 py-2.5 ac-chrome-label font-medium text-white shadow-xs transition hover:opacity-90 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] active:opacity-100"
                >
                  <Plus size={14} />
                  <span>Create Session</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}
