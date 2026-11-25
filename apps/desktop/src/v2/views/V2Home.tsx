import React, { useEffect, useMemo, useState } from "react";
import {
  Binary,
  FileText,
  Folder,
  GitMerge,
  Image,
  RefreshCw,
  Search,
  Sparkles,
  Table2,
  Trash2,
} from "lucide-react";
import { deleteSession, sessionList, sessionLoad } from "../../ipc/session";
import type { SessionEntry } from "../../ipc/session";
import type { SessionKind } from "../types";

export interface V2HomeProps {
  readonly onNewSession: (kind: SessionKind, left?: string, right?: string, title?: string) => void;
  readonly onOpenSessionManager: () => void;
}

interface SessionCardDef {
  readonly kind: SessionKind;
  readonly title: string;
  readonly subtitle: string;
  readonly icon: React.ComponentType<{ size?: number; strokeWidth?: number }>;
  readonly accent: string;
}

const SESSION_CARDS: readonly SessionCardDef[] = [
  {
    kind: "text",
    title: "Text Compare",
    subtitle: "Side-by-side line and word diff",
    icon: FileText,
    accent: "#2563eb",
  },
  {
    kind: "folder",
    title: "Folder Compare",
    subtitle: "Directories & archives",
    icon: Folder,
    accent: "#d97706",
  },
  {
    kind: "table",
    title: "Table Compare",
    subtitle: "CSV and structured data grids",
    icon: Table2,
    accent: "#059669",
  },
  {
    kind: "hex",
    title: "Hex Compare",
    subtitle: "Binary bytes and ASCII offsets",
    icon: Binary,
    accent: "#7c3aed",
  },
  {
    kind: "merge",
    title: "3-Way Text Merge",
    subtitle: "Resolve line conflicts with base",
    icon: GitMerge,
    accent: "#dc2626",
  },
  {
    kind: "folderMerge",
    title: "3-Way Folder Merge",
    subtitle: "Merge directory trees over base",
    icon: Folder,
    accent: "#ea580c",
  },
  {
    kind: "sync",
    title: "Folder Sync",
    subtitle: "Mirror and update directory trees",
    icon: RefreshCw,
    accent: "#0891b2",
  },
  {
    kind: "media",
    title: "Media Compare",
    subtitle: "Images, heatmaps, and audio",
    icon: Image,
    accent: "#db2777",
  },
];

export default function V2Home({
  onNewSession,
  onOpenSessionManager,
}: V2HomeProps): React.JSX.Element {
  const [sessions, setSessions] = useState<readonly SessionEntry[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);

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
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sessions;
    return sessions.filter((s) => (s.name ?? s.id).toLowerCase().includes(q));
  }, [sessions, query]);

  const handleOpenSaved = async (entry: SessionEntry): Promise<void> => {
    try {
      const doc = await sessionLoad(entry.id);
      const first = doc.tabs[0];
      const rawKind = (first?.session_type ?? "text").toLowerCase();
      const kindMap: Record<string, SessionKind> = {
        text: "text",
        folder: "folder",
        table: "table",
        hex: "hex",
        merge: "merge",
        foldermerge: "folderMerge",
        sync: "sync",
        media: "media",
      };
      const kind: SessionKind = kindMap[rawKind] ?? "text";
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

  return (
    <div data-testid="v2/home" className="flex h-full min-h-0 w-full overflow-hidden bg-[var(--v2-bg)]">
      {/* Left Saved Sessions Sidebar (Beyond Compare style) */}
      <aside className="flex w-[260px] shrink-0 flex-col border-r border-[var(--v2-border)] bg-[var(--v2-sidebar)]">
        <div className="flex items-center justify-between border-b border-[var(--v2-border-subtle)] px-3.5 py-2.5">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-[var(--v2-fg-muted)]">
            Saved Sessions
          </span>
          <span className="rounded-full bg-[var(--v2-seg-track)] px-1.5 py-0.5 text-[10px] font-medium text-[var(--v2-fg-secondary)]">
            {sessions.length}
          </span>
        </div>

        <div className="border-b border-[var(--v2-border-subtle)] px-3 py-2">
          <div className="relative">
            <Search
              size={12}
              className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
            />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Filter sessions…"
              className="v2-input w-full"
              style={{ paddingLeft: 26 }}
            />
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-2">
          {loading ? (
            <p className="px-2 py-4 text-center text-[12px] text-[var(--v2-fg-muted)]">
              Loading sessions…
            </p>
          ) : filtered.length === 0 ? (
            <div className="px-3 py-8 text-center">
              <p className="text-[12px] font-medium text-[var(--v2-fg-secondary)]">
                No saved sessions
              </p>
              <p className="mt-1 text-[11px] text-[var(--v2-fg-muted)]">
                Start a comparison on the right or save your active workspace.
              </p>
            </div>
          ) : (
            <ul className="space-y-0.5">
              {filtered.map((entry) => (
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
                    className="group flex cursor-pointer items-center justify-between rounded-[6px] px-2.5 py-1.5 text-[12px] text-[var(--v2-fg)] hover:bg-[var(--v2-btn-hover)]"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-medium">{entry.name ?? entry.id}</div>
                      <div className="text-[10px] text-[var(--v2-fg-muted)]">
                        {entry.tabs} {entry.tabs === 1 ? "tab" : "tabs"}
                      </div>
                    </div>
                    <button
                      type="button"
                      title="Delete session"
                      onClick={(e) => void handleDeleteSaved(e, entry.id)}
                      className="ml-2 hidden h-5 w-5 items-center justify-center rounded text-[var(--v2-fg-muted)] hover:bg-[var(--v2-btn-active)] hover:text-red-600 group-hover:inline-flex"
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="border-t border-[var(--v2-border-subtle)] p-2.5">
          <button
            type="button"
            onClick={onOpenSessionManager}
            className="v2-btn w-full justify-center"
          >
            Manage Sessions…
          </button>
        </div>
      </aside>

      {/* Right New Comparison Workspace */}
      <main className="flex flex-1 flex-col overflow-y-auto px-10 py-8">
        <div className="mx-auto w-full max-w-4xl">
          <div className="mb-6 flex items-end justify-between">
            <div>
              <h1 className="text-[22px] font-semibold tracking-tight text-[var(--v2-fg)]">
                AutoCompare
              </h1>
              <p className="mt-1 text-[13px] text-[var(--v2-fg-muted)]">
                Choose a comparison type or open a saved session to begin.
              </p>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => onNewSession("text", "", "", "Demo Text Compare")}
                className="v2-btn v2-btn-primary"
              >
                <Sparkles size={13} />
                Try Demo Text Diff
              </button>
              <button
                type="button"
                onClick={() => onNewSession("folder", "", "", "Demo Folder Compare")}
                className="v2-btn"
              >
                Demo Folder Diff
              </button>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
            {SESSION_CARDS.map((card) => {
              const Icon = card.icon;
              return (
                <button
                  key={card.kind}
                  type="button"
                  data-testid={`v2/home-card-${card.kind}`}
                  onClick={() => onNewSession(card.kind)}
                  className="group flex items-start gap-3 rounded-[9px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-3.5 text-left shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition hover:border-[var(--v2-accent)] hover:shadow-[0_2px_8px_rgba(10,132,255,0.12)]"
                >
                  <div
                    className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-[7px]"
                    style={{ backgroundColor: `${card.accent}18`, color: card.accent }}
                  >
                    <Icon size={17} strokeWidth={1.8} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="text-[13px] font-semibold text-[var(--v2-fg)] group-hover:text-[var(--v2-accent)]">
                      {card.title}
                    </div>
                    <div className="mt-0.5 text-[11px] leading-snug text-[var(--v2-fg-muted)]">
                      {card.subtitle}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </main>
    </div>
  );
}
