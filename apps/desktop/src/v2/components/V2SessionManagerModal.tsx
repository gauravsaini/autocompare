import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Lock, Search, Star, Trash2, Unlock } from "lucide-react";
import { deleteSession, sessionList, sessionLoad, sessionSave } from "../../ipc/session";
import type { SessionEntry } from "../../ipc/session";
import {
  loadPinnedSessionIds,
  savePinnedSessionIds,
  sortSessionsWithPins,
  togglePinnedId,
} from "../utils/sessionPrefs";
import { resolveComparatorId } from "../plugins/registry";
import V2Modal from "./V2Modal";
import type { SessionKind } from "../types";

export interface V2SessionManagerModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onOpenSession: (kind: SessionKind, left?: string, right?: string, title?: string) => void;
  readonly onSessionsChanged?: () => void;
}

export default function V2SessionManagerModal({
  open,
  onClose,
  onOpenSession,
  onSessionsChanged,
}: V2SessionManagerModalProps): React.JSX.Element {
  const [sessions, setSessions] = useState<readonly SessionEntry[]>([]);
  const [pinnedIds, setPinnedIds] = useState<string[]>(() => loadPinnedSessionIds());
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);

  const loadList = useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const list = await sessionList();
      setSessions(Array.isArray(list) ? list : []);
      setPinnedIds(loadPinnedSessionIds());
    } catch {
      setSessions([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) void loadList();
  }, [open, loadList]);

  const handleTogglePin = (id: string): void => {
    setPinnedIds((prev) => {
      const next = togglePinnedId(prev, id);
      savePinnedSessionIds(next);
      return next;
    });
    onSessionsChanged?.();
  };

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const matched = q ? sessions.filter((s) => (s.name ?? s.id).toLowerCase().includes(q)) : sessions;
    return sortSessionsWithPins(matched, pinnedIds);
  }, [sessions, query, pinnedIds]);

  const handleOpen = async (entry: SessionEntry): Promise<void> => {
    try {
      const doc = await sessionLoad(entry.id);
      const first = doc.tabs[0];
      const kind = resolveComparatorId(first?.session_type);
      onOpenSession(
        kind,
        first?.left_path ?? "",
        first?.right_path ?? "",
        doc.name ?? entry.name ?? first?.title ?? `${kind} session`,
      );
      onClose();
    } catch {
      onOpenSession("text", "", "", entry.name ?? entry.id);
      onClose();
    }
  };

  const handleToggleLock = async (entry: SessionEntry): Promise<void> => {
    setBusyId(entry.id);
    try {
      const doc = await sessionLoad(entry.id);
      const nextLocked = !entry.locked;
      await deleteSession(entry.id);
      await sessionSave({ ...doc, locked: nextLocked });
      await loadList();
      onSessionsChanged?.();
    } catch {
      // Best-effort
    } finally {
      setBusyId(null);
    }
  };

  const handleCommitRename = async (entry: SessionEntry): Promise<void> => {
    const nextName = renameDraft.trim();
    setRenamingId(null);
    if (!nextName || nextName === (entry.name ?? entry.id)) return;
    setBusyId(entry.id);
    try {
      const doc = await sessionLoad(entry.id);
      await deleteSession(entry.id);
      await sessionSave({ ...doc, name: nextName });
      await loadList();
      onSessionsChanged?.();
    } catch {
      // Best-effort
    } finally {
      setBusyId(null);
    }
  };

  const handleDelete = async (entry: SessionEntry): Promise<void> => {
    if (entry.locked) return;
    setBusyId(entry.id);
    try {
      await deleteSession(entry.id);
      setSessions((prev) => prev.filter((s) => s.id !== entry.id));
      onSessionsChanged?.();
    } catch {
      // Best-effort
    } finally {
      setBusyId(null);
    }
  };

  return (
    <V2Modal
      open={open}
      title="Session Manager"
      subtitle="Open, rename, lock, or remove saved comparison sessions"
      widthClass="max-w-lg"
      onClose={onClose}
      footer={
        <button type="button" onClick={onClose} className="v2-btn v2-btn-primary">
          Done
        </button>
      }
    >
      <div className="space-y-3.5 text-[12px]">
        <div className="relative flex items-center">
          <Search
            size={14}
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
          />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search saved sessions…"
            className="h-9 w-full rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] pl-9 pr-3 text-[12.5px] text-[var(--v2-fg)] outline-none transition-colors placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:bg-[var(--v2-surface)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
          />
        </div>

        <div className="max-h-72 overflow-y-auto rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)]/60">
          {loading ? (
            <p className="px-3 py-6 text-center text-[var(--v2-fg-muted)]">Loading sessions…</p>
          ) : filtered.length === 0 ? (
            <p className="px-3 py-6 text-center text-[var(--v2-fg-muted)]">No saved sessions found.</p>
          ) : (
            <ul className="divide-y divide-[var(--v2-border-subtle)]">
              {filtered.map((entry) => {
                const isRenaming = renamingId === entry.id;
                const isBusy = busyId === entry.id;
                return (
                  <li
                    key={entry.id}
                    className="flex items-center justify-between gap-2 px-3 py-2 hover:bg-[var(--v2-btn-hover)]/40"
                  >
                    <div className="min-w-0 flex-1">
                      {isRenaming ? (
                        <input
                          autoFocus
                          value={renameDraft}
                          onChange={(e) => setRenameDraft(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") void handleCommitRename(entry);
                            if (e.key === "Escape") setRenamingId(null);
                          }}
                          onBlur={() => void handleCommitRename(entry)}
                          className="v2-input w-full"
                        />
                      ) : (
                        <button
                          type="button"
                          onClick={() => void handleOpen(entry)}
                          className="flex w-full items-center gap-1.5 truncate text-left font-medium text-[var(--v2-fg)] hover:text-[var(--v2-accent)]"
                        >
                          {entry.locked ? (
                            <Lock size={11} className="shrink-0 text-[var(--v2-amber)]" />
                          ) : null}
                          <span className="truncate">{entry.name ?? entry.id}</span>
                        </button>
                      )}
                      <div className="text-[10.5px] text-[var(--v2-fg-muted)]">
                        {entry.tabs} {entry.tabs === 1 ? "tab" : "tabs"}
                        {entry.locked ? " · Locked" : ""}
                      </div>
                    </div>

                    <div className="flex shrink-0 items-center gap-1">
                      <button
                        type="button"
                        disabled={isBusy}
                        title={pinnedIds.includes(entry.id) ? "Unpin session" : "Pin session"}
                        onClick={() => handleTogglePin(entry.id)}
                        className={`v2-btn h-[22px] px-1.5 ${pinnedIds.includes(entry.id) ? "text-[var(--v2-amber)]" : ""}`}
                      >
                        <Star size={11} fill={pinnedIds.includes(entry.id) ? "currentColor" : "none"} />
                      </button>
                      <button
                        type="button"
                        disabled={isBusy}
                        onClick={() => {
                          setRenamingId(entry.id);
                          setRenameDraft(entry.name ?? entry.id);
                        }}
                        className="v2-btn h-[22px] px-2 text-[11px]"
                      >
                        Rename
                      </button>
                      <button
                        type="button"
                        disabled={isBusy}
                        title={entry.locked ? "Unlock session" : "Lock session"}
                        onClick={() => void handleToggleLock(entry)}
                        className="v2-btn h-[22px] px-1.5"
                      >
                        {entry.locked ? <Unlock size={11} /> : <Lock size={11} />}
                      </button>
                      <button
                        type="button"
                        disabled={isBusy || entry.locked}
                        title={entry.locked ? "Unlock before deleting" : "Delete session"}
                        onClick={() => void handleDelete(entry)}
                        className="v2-btn h-[22px] px-1.5 text-[var(--v2-del-fg)]"
                      >
                        <Trash2 size={11} />
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </V2Modal>
  );
}
