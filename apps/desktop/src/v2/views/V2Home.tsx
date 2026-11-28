import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowRight,
  Bookmark,
  CornerDownLeft,
  FileText,
  FolderOpen,
  Lock,
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  Sparkles,
  Star,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import appLogo from "../../assets/logo.png";
import { deleteSession, sessionList, sessionLoad } from "../../ipc/session";
import type { SessionEntry } from "../../ipc/session";
import {
  getComparator,
  inferComparatorForPaths,
  listComparators,
  resolveComparatorId,
} from "../plugins/registry";
import {
  clampSidebarWidth,
  loadHomeSidebarPrefs,
  saveHomeSidebarPrefs,
} from "../utils/homePrefs";
import {
  loadPinnedSessionIds,
  savePinnedSessionIds,
  sortSessionsWithPins,
  togglePinnedId,
} from "../utils/sessionPrefs";
import { matchSavedSessions, parseSpotlightPaths } from "../utils/spotlightSearch";
import type { SessionKind } from "../types";

export interface V2HomeProps {
  readonly demoMode?: boolean;
  readonly onNewSession: (kind: SessionKind, left?: string, right?: string, title?: string) => void;
  readonly onOpenSessionManager: () => void;
  readonly onOpenNewSessionModal?: () => void;
  readonly onOpenCommandPalette?: () => void;
  readonly openComparisonsCount?: number;
  readonly refreshSignal?: number;
}

function inferSessionBadge(name: string): {
  icon: React.ComponentType<{ size?: number; strokeWidth?: number }>;
  accent: string;
} {
  const n = name.toLowerCase();
  for (const plugin of listComparators()) {
    if (plugin.keywords?.some((kw) => n.includes(kw))) {
      return { icon: plugin.icon, accent: plugin.accent };
    }
  }
  return { icon: FileText, accent: "#1d4ed8" };
}

export default function V2Home({
  demoMode = false,
  onNewSession,
  onOpenSessionManager,
  onOpenNewSessionModal,
  onOpenCommandPalette,
  openComparisonsCount = 0,
  refreshSignal = 0,
}: V2HomeProps): React.JSX.Element {
  const comparators = listComparators();
  const [sessions, setSessions] = useState<readonly SessionEntry[]>([]);
  const [pinnedIds, setPinnedIds] = useState<string[]>(() => loadPinnedSessionIds());
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [sessionQuery, setSessionQuery] = useState("");
  const [spotlightQuery, setSpotlightQuery] = useState("");
  const [spotlightIdx, setSpotlightIdx] = useState(0);
  const [spotlightNavActive, setSpotlightNavActive] = useState(false);
  const [loading, setLoading] = useState(true);

  // Persisted dockable + resizable left sidebar
  const initialPrefs = useMemo(() => loadHomeSidebarPrefs(), []);
  const [sidebarOpen, setSidebarOpen] = useState(initialPrefs.open);
  const [sidebarWidth, setSidebarWidth] = useState(initialPrefs.width);
  const sidebarWidthRef = useRef(sidebarWidth);
  sidebarWidthRef.current = sidebarWidth;
  const sidebarOpenRef = useRef(sidebarOpen);
  sidebarOpenRef.current = sidebarOpen;
  const resizingRef = useRef(false);

  const refreshSessions = async (): Promise<void> => {
    setLoading(true);
    try {
      const list = await sessionList();
      setSessions(Array.isArray(list) ? list : []);
    } catch {
      setSessions([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refreshSessions();
    setPinnedIds(loadPinnedSessionIds());
  }, [refreshSignal]);

  // Persist dock open/collapsed state immediately
  useEffect(() => {
    saveHomeSidebarPrefs({ open: sidebarOpen, width: sidebarWidthRef.current });
  }, [sidebarOpen]);

  useEffect(() => {
    const onMove = (e: PointerEvent): void => {
      if (!resizingRef.current) return;
      const nextWidth = clampSidebarWidth(e.clientX);
      sidebarWidthRef.current = nextWidth;
      setSidebarWidth(nextWidth);
    };
    const onUp = (): void => {
      if (!resizingRef.current) return;
      resizingRef.current = false;
      document.body.style.userSelect = "";
      saveHomeSidebarPrefs({ open: sidebarOpenRef.current, width: sidebarWidthRef.current });
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, []);

  const handleTogglePin = (e: React.MouseEvent, id: string): void => {
    e.stopPropagation();
    setPinnedIds((prev) => {
      const next = togglePinnedId(prev, id);
      savePinnedSessionIds(next);
      return next;
    });
  };

  const filteredSessions = useMemo(() => {
    const q = sessionQuery.trim().toLowerCase();
    const base = pinnedOnly ? sessions.filter((s) => pinnedIds.includes(s.id)) : sessions;
    const matched = q ? base.filter((s) => (s.name ?? s.id).toLowerCase().includes(q)) : base;
    return sortSessionsWithPins(matched, pinnedIds);
  }, [sessions, sessionQuery, pinnedOnly, pinnedIds]);

  const parsedPaths = useMemo(
    () => parseSpotlightPaths(spotlightQuery, inferComparatorForPaths),
    [spotlightQuery],
  );

  const spotlightSessions = useMemo(
    () => (parsedPaths ? [] : matchSavedSessions(sessions, spotlightQuery, 4)),
    [sessions, spotlightQuery, parsedPaths],
  );

  const filteredPlugins = useMemo(() => {
    const q = spotlightQuery.trim().toLowerCase();
    if (!q || parsedPaths) return comparators;
    return comparators.filter((p) => {
      const hay = [
        p.id,
        p.title,
        p.subtitle,
        ...(p.extensions ?? []),
        ...(p.keywords ?? []),
      ]
        .join(" ")
        .toLowerCase();
      return hay.includes(q);
    });
  }, [comparators, spotlightQuery, parsedPaths]);

  const totalSpotlightItems = spotlightSessions.length + filteredPlugins.length;

  useEffect(() => {
    setSpotlightIdx(0);
    setSpotlightNavActive(spotlightQuery.trim().length > 0);
  }, [spotlightQuery]);

  const pinnedCount = useMemo(
    () => pinnedIds.filter((id) => sessions.some((s) => s.id === id)).length,
    [pinnedIds, sessions],
  );

  const handleOpenSaved = async (entry: SessionEntry): Promise<void> => {
    try {
      const doc = await sessionLoad(entry.id);
      const first = doc.tabs[0];
      const kind = resolveComparatorId(first?.session_type);
      onNewSession(
        kind,
        first?.left_path ?? "",
        first?.right_path ?? "",
        doc.name ?? entry.name ?? first?.title ?? `${kind} session`,
      );
    } catch {
      onNewSession("text", "", "", entry.name ?? entry.id);
    }
  };

  const handleDeleteSaved = async (e: React.MouseEvent, id: string): Promise<void> => {
    e.stopPropagation();
    try {
      await deleteSession(id);
    } catch {
      // Best-effort
    }
    setSessions((prev) => prev.filter((s) => s.id !== id));
  };

  const launchParsedPaths = (): void => {
    if (!parsedPaths) return;
    onNewSession(parsedPaths.inferredKind, parsedPaths.left, parsedPaths.right);
    setSpotlightQuery("");
  };

  const inferredPlugin = parsedPaths ? getComparator(parsedPaths.inferredKind) : undefined;

  // When a 4-column desktop grid ends with a 2-card final row (e.g. 10 plugins), span both across 2 cols
  const spanLastTwo = filteredPlugins.length >= 6 && filteredPlugins.length % 4 === 2;

  return (
    <div data-testid="v2/home" className="relative flex h-full min-h-0 w-full overflow-hidden bg-[var(--v2-bg)]">
      {/* Dockable Left Saved Sessions Sidebar */}
      {sidebarOpen ? (
        <>
          <aside
            data-testid="v2/home-sidebar"
            style={{ width: sidebarWidth }}
            className="flex shrink-0 flex-col justify-between border-r border-[var(--v2-border)] bg-[var(--v2-sidebar)] p-4"
          >
            <div className="flex min-h-0 flex-1 flex-col gap-3.5">
              {/* Sidebar Header with Dock Toggle */}
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <span className="v2-mono text-[11px] font-semibold uppercase tracking-wider text-[var(--v2-fg-muted)]">
                    Saved Sessions
                  </span>
                  <span className="v2-mono rounded-full bg-[var(--v2-surface-subtle)] px-1.5 py-0.5 text-[11px] text-[var(--v2-fg-secondary)]">
                    {sessions.length}
                  </span>
                </div>
                <button
                  type="button"
                  data-testid="v2/sidebar-undock"
                  aria-label="Collapse sidebar"
                  title="Undock / Collapse Sidebar"
                  onClick={() => setSidebarOpen(false)}
                  className="flex h-7 w-7 items-center justify-center rounded-lg text-[var(--v2-fg-muted)] transition-colors hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                >
                  <PanelLeftClose size={16} strokeWidth={1.8} />
                </button>
              </div>

              {/* Filter Input */}
              <div className="relative">
                <Search
                  size={14}
                  className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
                />
                <input
                  type="text"
                  value={sessionQuery}
                  onChange={(e) => setSessionQuery(e.target.value)}
                  placeholder="Filter sessions…"
                  className="h-9 w-full rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] pl-9 pr-3 text-[12px] text-[var(--v2-fg)] outline-none placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
                />
              </div>

              {/* Segmented All / Pinned Filter */}
              <div className="grid grid-cols-2 gap-1 rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] p-1">
                <button
                  type="button"
                  onClick={() => setPinnedOnly(false)}
                  className={`h-7 rounded-lg text-[11.5px] font-medium transition-colors ${
                    !pinnedOnly
                      ? "bg-[var(--v2-accent)] text-[var(--v2-accent-fg)] shadow-sm"
                      : "text-[var(--v2-fg-muted)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  All
                </button>
                <button
                  type="button"
                  onClick={() => setPinnedOnly(true)}
                  className={`flex h-7 items-center justify-center gap-1 rounded-lg text-[11.5px] font-medium transition-colors ${
                    pinnedOnly
                      ? "bg-[var(--v2-accent)] text-[var(--v2-accent-fg)] shadow-sm"
                      : "text-[var(--v2-fg-muted)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  <Star size={11} />
                  <span>Pinned ({pinnedCount})</span>
                </button>
              </div>

              {/* Saved Sessions List / Empty State */}
              <div className="min-h-0 flex-1 overflow-y-auto pr-0.5">
                {loading ? (
                  <p className="px-2 py-6 text-center text-[12px] text-[var(--v2-fg-muted)]">
                    Loading sessions…
                  </p>
                ) : filteredSessions.length === 0 ? (
                  <div className="mt-4 flex flex-col items-center gap-1.5 rounded-2xl border border-dashed border-[var(--v2-border-strong)] bg-[var(--v2-bg)]/70 p-5 text-center">
                    <Bookmark size={20} strokeWidth={1.6} className="text-[var(--v2-fg-muted)]" />
                    <p className="text-[12px] font-semibold text-[var(--v2-fg)]">
                      {pinnedOnly ? "No pinned sessions" : "No saved sessions"}
                    </p>
                    <p className="text-[11px] leading-relaxed text-[var(--v2-fg-muted)]">
                      {pinnedOnly
                        ? "Click the star icon on any saved session to pin it here."
                        : "Start a comparison on the right or save your active workspace."}
                    </p>
                  </div>
                ) : (
                  <ul className="space-y-1">
                    {filteredSessions.map((entry) => {
                      const badge = inferSessionBadge(entry.name ?? entry.id);
                      const BadgeIcon = badge.icon;
                      const isPinned = pinnedIds.includes(entry.id);
                      return (
                        <li key={entry.id}>
                          <div
                            role="button"
                            tabIndex={0}
                            onClick={() => void handleOpenSaved(entry)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                e.preventDefault();
                                void handleOpenSaved(entry);
                              }
                            }}
                            className="group flex cursor-pointer items-center justify-between gap-2.5 rounded-xl px-2.5 py-2 text-[12px] text-[var(--v2-fg)] transition-colors hover:bg-[var(--v2-surface-subtle)]"
                          >
                            <span
                              className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg"
                              style={{ backgroundColor: `${badge.accent}18`, color: badge.accent }}
                            >
                              <BadgeIcon size={14} strokeWidth={1.8} />
                            </span>
                            <div className="min-w-0 flex-1">
                              <div className="flex items-center gap-1 truncate font-medium">
                                {entry.locked ? (
                                  <Lock size={10} className="shrink-0 text-[var(--v2-amber)]" />
                                ) : null}
                                <span className="truncate">{entry.name ?? entry.id}</span>
                              </div>
                              <div className="v2-mono text-[10px] text-[var(--v2-fg-muted)]">
                                {entry.tabs} {entry.tabs === 1 ? "tab" : "tabs"}
                                {entry.locked ? " · Locked" : ""}
                              </div>
                            </div>
                            <div className="flex shrink-0 items-center gap-0.5">
                              <button
                                type="button"
                                title={isPinned ? "Unpin session" : "Pin session"}
                                onClick={(e) => handleTogglePin(e, entry.id)}
                                className={`inline-flex h-6 w-6 items-center justify-center rounded-lg hover:bg-[var(--v2-btn-active)] ${
                                  isPinned
                                    ? "text-[var(--v2-amber)]"
                                    : "hidden text-[var(--v2-fg-muted)] group-hover:inline-flex"
                                }`}
                              >
                                <Star size={11} fill={isPinned ? "currentColor" : "none"} />
                              </button>
                              {!entry.locked ? (
                                <button
                                  type="button"
                                  title="Delete session"
                                  onClick={(e) => void handleDeleteSaved(e, entry.id)}
                                  className="hidden h-6 w-6 shrink-0 items-center justify-center rounded-lg text-[var(--v2-fg-muted)] hover:bg-[var(--v2-btn-active)] hover:text-[var(--v2-del-fg)] group-hover:inline-flex"
                                >
                                  <Trash2 size={12} />
                                </button>
                              ) : null}
                            </div>
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </div>
            </div>

            {/* Sidebar Footer */}
            <div className="mt-3 flex flex-col gap-2.5 border-t border-[var(--v2-border-subtle)] pt-3">
              <button
                type="button"
                onClick={onOpenSessionManager}
                className="h-9 w-full rounded-xl bg-[var(--v2-surface-subtle)] text-[12px] font-medium text-[var(--v2-fg)] transition-colors hover:bg-[var(--v2-btn-active)]"
              >
                Manage Sessions…
              </button>
              <div className="v2-mono flex items-center justify-center text-[11px] text-[var(--v2-fg-muted)]">
                <span>
                  {openComparisonsCount} open {openComparisonsCount === 1 ? "comparison" : "comparisons"}
                </span>
              </div>
              <p className="text-center text-[11px] text-[var(--v2-fg-muted)]">
                With ❤️ from Melbourne
              </p>
            </div>
          </aside>

          {/* Draggable Sidebar Resize Handle */}
          <div
            id="sidebarResizer"
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize sidebar"
            onPointerDown={(e) => {
              resizingRef.current = true;
              (e.currentTarget as HTMLDivElement).setPointerCapture(e.pointerId);
              document.body.style.userSelect = "none";
            }}
            className="group flex w-1.5 shrink-0 cursor-col-resize items-center justify-center bg-transparent transition-colors hover:bg-[var(--v2-accent)]/20 active:bg-[var(--v2-accent)]/35"
          >
            <div className="h-8 w-0.5 rounded-full bg-[var(--v2-border-strong)] opacity-0 transition-opacity group-hover:opacity-100" />
          </div>
        </>
      ) : (
        <button
          type="button"
          data-testid="v2/sidebar-dock"
          aria-label="Expand sidebar"
          title="Dock / Expand Saved Sessions Sidebar"
          onClick={() => setSidebarOpen(true)}
          className="absolute left-4 top-4 z-10 flex h-9 w-9 items-center justify-center rounded-xl border border-[var(--v2-border)] bg-[var(--v2-surface)] text-[var(--v2-fg-muted)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:text-[var(--v2-fg)]"
        >
          <PanelLeftOpen size={17} strokeWidth={1.8} />
        </button>
      )}

      {/* Main Spotlight Launcher Workspace */}
      <main className="flex flex-1 flex-col overflow-y-auto px-6 py-8">
        <div className="mx-auto flex w-full max-w-5xl flex-col items-center gap-7">
          {/* Centered Impactful Hero with EXISTING AutoCompare Logo */}
          <div className="flex flex-col items-center gap-3 pt-1 text-center">
            <img
              src={appLogo}
              alt="AutoCompare logo"
              width={64}
              height={64}
              className="h-16 w-16 rounded-2xl shadow-lg"
            />
            <div className="flex flex-col gap-1">
              <h1 className="v2-display text-[32px] font-extrabold leading-tight tracking-tight text-[var(--v2-fg)]">
                AutoCompare
              </h1>
              <p className="text-[13.5px] text-[var(--v2-fg-muted)]">
                Choose a comparison plugin, search saved sessions, or paste paths to launch
              </p>
            </div>
          </div>

          {/* Mac Spotlight-style Search Bar + Smart Results + Quick Launch Pills */}
          <div className="flex w-full max-w-2xl flex-col gap-3">
            <div
              style={{ boxShadow: "var(--v2-shadow-spotlight)" }}
              className="relative flex items-center rounded-2xl border border-[var(--v2-border-strong)] bg-[var(--v2-surface)] transition-shadow focus-within:border-[var(--v2-accent)] focus-within:ring-2 focus-within:ring-[var(--v2-accent-ring)]"
            >
              <Search size={19} className="ml-4 shrink-0 text-[var(--v2-fg-muted)]" />
              <input
                type="text"
                data-testid="v2/spotlight-input"
                value={spotlightQuery}
                onChange={(e) => setSpotlightQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "ArrowDown" && !parsedPaths && totalSpotlightItems > 0) {
                    e.preventDefault();
                    setSpotlightNavActive(true);
                    setSpotlightIdx((prev) => (prev + 1) % totalSpotlightItems);
                  } else if (e.key === "ArrowUp" && !parsedPaths && totalSpotlightItems > 0) {
                    e.preventDefault();
                    setSpotlightNavActive(true);
                    setSpotlightIdx((prev) => (prev - 1 + totalSpotlightItems) % totalSpotlightItems);
                  } else if (e.key === "Enter") {
                    if (parsedPaths) {
                      e.preventDefault();
                      launchParsedPaths();
                    } else if (totalSpotlightItems > 0) {
                      e.preventDefault();
                      const safeIdx = Math.min(spotlightIdx, totalSpotlightItems - 1);
                      if (safeIdx < spotlightSessions.length) {
                        void handleOpenSaved(spotlightSessions[safeIdx]);
                      } else {
                        const plugin = filteredPlugins[safeIdx - spotlightSessions.length];
                        if (plugin) onNewSession(plugin.id);
                      }
                    }
                  } else if (e.key === "Escape") {
                    if (spotlightQuery) {
                      e.preventDefault();
                      setSpotlightQuery("");
                    } else if (spotlightNavActive) {
                      setSpotlightNavActive(false);
                    }
                  }
                }}
                placeholder="Search plugins, saved sessions, or paste paths (e.g. /a.json /b.json)…"
                className="h-13 w-full bg-transparent px-3.5 text-[14.5px] text-[var(--v2-fg)] outline-none placeholder:text-[var(--v2-fg-muted)]"
              />
              <div className="flex shrink-0 items-center gap-1.5 pr-3.5">
                {spotlightQuery ? (
                  <button
                    type="button"
                    aria-label="Clear search"
                    onClick={() => setSpotlightQuery("")}
                    className="flex h-6 w-6 items-center justify-center rounded-full text-[var(--v2-fg-muted)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                  >
                    <X size={13} />
                  </button>
                ) : null}
                <button
                  type="button"
                  title="Open Command Palette (⌘K)"
                  onClick={() => onOpenCommandPalette?.()}
                  className="v2-mono rounded-lg border border-[var(--v2-border)] bg-[var(--v2-bg)] px-2 py-1 text-[11px] text-[var(--v2-fg-muted)] transition-colors hover:border-[var(--v2-border-strong)] hover:text-[var(--v2-fg)]"
                >
                  ⌘ K
                </button>
              </div>
            </div>

            {/* Direct Path-Pair Launch Banner when paths are pasted into Spotlight */}
            {parsedPaths ? (
              <button
                type="button"
                data-testid="v2/spotlight-paths-action"
                onClick={launchParsedPaths}
                className="flex w-full items-center justify-between gap-3 rounded-xl border border-[var(--v2-accent)]/50 bg-[var(--v2-accent-soft)] px-3.5 py-2.5 text-left transition-colors hover:border-[var(--v2-accent)]"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 text-[12px] font-semibold text-[var(--v2-accent)]">
                    <span>Open in {inferredPlugin?.title ?? parsedPaths.inferredKind}</span>
                  </div>
                  <div className="v2-mono mt-0.5 truncate text-[11px] text-[var(--v2-fg-secondary)]">
                    {parsedPaths.left}
                    {parsedPaths.right ? `  ↔  ${parsedPaths.right}` : "  (select right side in view)"}
                  </div>
                </div>
                <span className="v2-mono inline-flex shrink-0 items-center gap-1 rounded-lg bg-[var(--v2-accent)] px-2 py-1 text-[10.5px] font-medium text-[var(--v2-accent-fg)]">
                  <span>Launch</span>
                  <CornerDownLeft size={11} />
                </span>
              </button>
            ) : null}

            {/* Matching Saved Sessions directly inside Spotlight */}
            {spotlightSessions.length > 0 ? (
              <div className="rounded-xl border border-[var(--v2-border)] bg-[var(--v2-surface)] p-2 shadow-sm">
                <div className="px-2 pb-1 text-[10.5px] font-semibold uppercase tracking-wider text-[var(--v2-fg-muted)]">
                  Matching Saved Sessions
                </div>
                <div className="space-y-0.5">
                  {spotlightSessions.map((entry, sIdx) => {
                    const badge = inferSessionBadge(entry.name ?? entry.id);
                    const BadgeIcon = badge.icon;
                    const isActive = spotlightNavActive && !parsedPaths && spotlightIdx === sIdx;
                    return (
                      <button
                        key={entry.id}
                        type="button"
                        data-testid={`v2/spotlight-session-${entry.id}`}
                        data-spotlight-active={isActive ? "true" : undefined}
                        onMouseEnter={() => {
                          setSpotlightNavActive(true);
                          setSpotlightIdx(sIdx);
                        }}
                        onClick={() => void handleOpenSaved(entry)}
                        className={`flex w-full items-center justify-between gap-2.5 rounded-lg px-2 py-1.5 text-left text-[12px] transition-colors ${
                          isActive
                            ? "bg-[var(--v2-accent-soft)] text-[var(--v2-accent)] ring-1 ring-[var(--v2-accent)]/50"
                            : "text-[var(--v2-fg)] hover:bg-[var(--v2-surface-subtle)]"
                        }`}
                      >
                        <div className="flex min-w-0 items-center gap-2">
                          <span
                            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md"
                            style={{ backgroundColor: `${badge.accent}18`, color: badge.accent }}
                          >
                            <BadgeIcon size={13} strokeWidth={1.8} />
                          </span>
                          <span className="truncate font-medium">{entry.name ?? entry.id}</span>
                        </div>
                        <span className="v2-mono shrink-0 text-[10.5px] text-[var(--v2-fg-muted)]">
                          {entry.tabs} {entry.tabs === 1 ? "tab" : "tabs"}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : null}

            {/* Quick Launch Action Pills (Demo actions visible only when Demo Mode is ON) */}
            <div className="flex flex-wrap items-center justify-center gap-2.5">
              {demoMode ? (
                <>
                  <button
                    type="button"
                    data-testid="v2/demo-text-btn"
                    aria-label="Try Demo Text Diff"
                    title="Try Demo Text Diff"
                    onClick={() => onNewSession("text", "", "", "Demo Text Compare")}
                    className="flex h-9 items-center gap-1.5 rounded-full bg-[var(--v2-accent)] px-3.5 text-[12px] font-medium text-[var(--v2-accent-fg)] shadow-sm transition-colors hover:bg-[var(--v2-accent-hover)]"
                  >
                    <Sparkles size={14} />
                    <span>Try Demo Text Diff</span>
                  </button>
                  <button
                    type="button"
                    data-testid="v2/demo-folder-btn"
                    aria-label="Demo Folder Diff"
                    title="Demo Folder Diff"
                    onClick={() => onNewSession("folder", "", "", "Demo Folder Compare")}
                    className="flex h-9 items-center gap-1.5 rounded-full border border-[var(--v2-border)] bg-[var(--v2-surface)] px-3.5 text-[12px] font-medium text-[var(--v2-fg-secondary)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                  >
                    <FolderOpen size={14} />
                    <span>Demo Folder Diff</span>
                  </button>
                </>
              ) : null}
              {onOpenNewSessionModal ? (
                <button
                  type="button"
                  aria-label="Open Files..."
                  title="Open New Session with Custom Paths…"
                  onClick={onOpenNewSessionModal}
                  className="flex h-9 items-center gap-1.5 rounded-full border border-[var(--v2-border)] bg-[var(--v2-surface)] px-3.5 text-[12px] font-medium text-[var(--v2-fg-secondary)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                >
                  <Upload size={14} />
                  <span>Open Paths…</span>
                </button>
              ) : null}
            </div>
          </div>

          {/* Registry-Driven Comparison Plugins Grid */}
          <section className="grid w-full grid-cols-1 gap-2.5 pt-1">
            <div className="flex items-center justify-between px-1">
              <h2 className="v2-display text-[11px] font-bold uppercase tracking-wider text-[var(--v2-fg-muted)]">
                Comparison Plugins
              </h2>
              <span className="v2-mono text-[11px] text-[var(--v2-fg-muted)]">
                {filteredPlugins.length} {filteredPlugins.length === 1 ? "engine" : "engines"}
              </span>
            </div>

            {filteredPlugins.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-[var(--v2-border-strong)] bg-[var(--v2-surface)]/60 py-10 text-center text-[12.5px] text-[var(--v2-fg-muted)]">
                No comparison plugins match &ldquo;{spotlightQuery}&rdquo;
              </div>
            ) : (
              <div className="grid grid-cols-1 items-stretch gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
                {filteredPlugins.map((plugin, idx) => {
                  const Icon = plugin.icon;
                  const isWide = spanLastTwo && idx >= filteredPlugins.length - 2;
                  const flatIdx = spotlightSessions.length + idx;
                  const isActive = spotlightNavActive && !parsedPaths && spotlightIdx === flatIdx;
                  return (
                    <button
                      key={plugin.id}
                      type="button"
                      data-testid={`v2/home-card-${plugin.id}`}
                      data-spotlight-active={isActive ? "true" : undefined}
                      onMouseEnter={() => {
                        if (spotlightNavActive) setSpotlightIdx(flatIdx);
                      }}
                      onClick={() => onNewSession(plugin.id)}
                      className={`group grid h-full rounded-xl border bg-[var(--v2-surface)] p-3 text-left transition-all hover:border-[var(--v2-accent)]/60 hover:shadow-md ${
                        isActive
                          ? "border-[var(--v2-accent)] ring-2 ring-[var(--v2-accent-ring)] shadow-md"
                          : "border-[var(--v2-border)]"
                      } ${
                        isWide
                          ? "grid-cols-[auto_1fr_auto] items-center gap-3 lg:col-span-2"
                          : "grid-cols-[auto_1fr] items-start gap-2.5"
                      }`}
                    >
                      <div
                        className="grid h-8 w-8 shrink-0 place-items-center rounded-lg"
                        style={{ backgroundColor: `${plugin.accent}18`, color: plugin.accent }}
                      >
                        <Icon size={17} strokeWidth={1.85} />
                      </div>
                      <div className="min-w-0">
                        <h3 className="v2-display text-[13px] font-semibold text-[var(--v2-fg)] transition-colors group-hover:text-[var(--v2-accent)]">
                          {plugin.title}
                        </h3>
                        <p className="mt-0.5 text-[11.5px] leading-snug text-[var(--v2-fg-muted)]">
                          {plugin.subtitle}
                        </p>
                      </div>
                      {isWide ? (
                        <ArrowRight
                          size={15}
                          className="shrink-0 text-[var(--v2-fg-muted)] transition-transform group-hover:translate-x-0.5 group-hover:text-[var(--v2-accent)]"
                        />
                      ) : null}
                    </button>
                  );
                })}
              </div>
            )}
          </section>
        </div>
      </main>
    </div>
  );
}
