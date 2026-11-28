import React, { useEffect, useMemo, useState } from "react";
import { Check, FolderOpen, Search, X } from "lucide-react";
import V2Modal from "./V2Modal";
import { listComparators } from "../plugins/registry";
import { browsePath } from "../utils/dialog";
import type { SessionKind } from "../types";

export interface V2NewSessionModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onCreate: (kind: SessionKind, left: string, right: string) => void;
}

const DIRECTORY_KINDS = new Set<SessionKind>(["folder", "folderMerge", "sync"]);

export default function V2NewSessionModal({
  open,
  onClose,
  onCreate,
}: V2NewSessionModalProps): React.JSX.Element {
  const comparators = listComparators();
  const [draftKind, setDraftKind] = useState<SessionKind>("text");
  const [query, setQuery] = useState("");
  const [draftLeft, setDraftLeft] = useState("");
  const [draftRight, setDraftRight] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return comparators;
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
  }, [comparators, query]);

  useEffect(() => {
    if (filtered.length > 0 && !filtered.some((p) => p.id === draftKind)) {
      setDraftKind(filtered[0].id);
    }
  }, [filtered, draftKind]);

  const stepSelection = (delta: number): void => {
    if (filtered.length === 0) return;
    const curIdx = filtered.findIndex((p) => p.id === draftKind);
    const base = curIdx >= 0 ? curIdx : 0;
    const nextIdx = (base + delta + filtered.length) % filtered.length;
    setDraftKind(filtered[nextIdx].id);
  };

  const handleOpen = (kind: SessionKind): void => {
    onCreate(kind, draftLeft.trim(), draftRight.trim());
    setDraftLeft("");
    setDraftRight("");
    setQuery("");
    onClose();
  };

  const selectedPlugin =
    comparators.find((p) => p.id === draftKind) ?? comparators[0];

  const isDirectoryKind = DIRECTORY_KINDS.has(draftKind);

  const handleBrowse = async (side: "left" | "right"): Promise<void> => {
    const picked = await browsePath(isDirectoryKind);
    if (!picked) return;
    if (side === "left") setDraftLeft(picked);
    else setDraftRight(picked);
  };

  // Span the last two cards across 2 columns on desktop when a 2-column-per-row or 4-col grid has a 2-item final row
  const spanLastTwo = filtered.length >= 6 && filtered.length % 2 === 0 && filtered.length % 3 === 1;

  return (
    <V2Modal
      open={open}
      title="New Comparison Session"
      subtitle="Select a comparator card (double-click to open immediately) or enter optional paths below"
      widthClass="max-w-3xl"
      onClose={onClose}
      footer={
        <div className="flex w-full items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-[11.5px] text-[var(--v2-fg-muted)]">
            {selectedPlugin ? (
              <>
                <span
                  className="inline-block h-2 w-2 rounded-full"
                  style={{ backgroundColor: selectedPlugin.accent }}
                />
                <span>
                  Selected: <strong className="font-semibold text-[var(--v2-fg)]">{selectedPlugin.title}</strong>
                </span>
              </>
            ) : (
              <span>Select a comparator</span>
            )}
          </div>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className="v2-btn h-8 rounded-xl px-3.5">
              Cancel
            </button>
            <button
              type="button"
              data-testid="v2/new-session-open-btn"
              onClick={() => handleOpen(draftKind)}
              className="v2-btn v2-btn-primary h-8 rounded-xl px-4"
            >
              Open Comparison
            </button>
          </div>
        </div>
      }
    >
      <div className="space-y-4 text-[12px]">
        {/* Spotlight-Style Filter Bar (proper left padding so icon never overlaps placeholder) */}
        <div className="relative flex items-center">
          <Search
            size={15}
            className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
          />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowRight") {
                e.preventDefault();
                stepSelection(1);
              } else if (e.key === "ArrowLeft") {
                e.preventDefault();
                stepSelection(-1);
              } else if (e.key === "ArrowDown") {
                e.preventDefault();
                stepSelection(2);
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                stepSelection(-2);
              } else if (e.key === "Enter" && filtered.length > 0) {
                e.preventDefault();
                const target = filtered.some((p) => p.id === draftKind) ? draftKind : filtered[0].id;
                handleOpen(target);
              }
            }}
            placeholder="Filter comparators (e.g. text, folder, json, toml, merge, hex)…"
            className="h-10 w-full rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] pl-10 pr-9 text-[12.5px] text-[var(--v2-fg)] outline-none transition-colors placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:bg-[var(--v2-surface)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
          />
          {query ? (
            <button
              type="button"
              aria-label="Clear filter"
              onClick={() => setQuery("")}
              className="absolute right-2.5 inline-flex h-6 w-6 items-center justify-center rounded-full text-[var(--v2-fg-muted)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
            >
              <X size={13} />
            </button>
          ) : null}
        </div>

        {/* Clean Unclipped Multi-Column Comparator Grid (all 10 cards visible comfortably) */}
        <div
          data-testid="v2/new-session-grid"
          className="grid max-h-[360px] grid-cols-1 gap-2.5 overflow-y-auto p-0.5 sm:grid-cols-2"
        >
          {filtered.map((plugin) => {
            const Icon = plugin.icon;
            const selected = draftKind === plugin.id;
            return (
              <button
                key={plugin.id}
                type="button"
                data-testid={`v2/new-session-card-${plugin.id}`}
                aria-pressed={selected}
                onClick={() => setDraftKind(plugin.id)}
                onDoubleClick={() => handleOpen(plugin.id)}
                className={`group relative flex items-center gap-3 rounded-xl border p-3 text-left transition-all ${
                  selected
                    ? "border-[var(--v2-accent)] bg-[var(--v2-accent-soft)]/70 ring-1 ring-[var(--v2-accent)]"
                    : "border-[var(--v2-border)] bg-[var(--v2-surface)] hover:border-[var(--v2-accent)]/50 hover:shadow-sm"
                } ${spanLastTwo ? "" : ""}`}
              >
                <span
                  className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg"
                  style={{
                    backgroundColor: `${plugin.accent}18`,
                    color: plugin.accent,
                  }}
                >
                  <Icon size={17} strokeWidth={1.9} />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center justify-between gap-2">
                    <span className="v2-display truncate text-[12.5px] font-semibold text-[var(--v2-fg)]">
                      {plugin.title}
                    </span>
                    <div className="flex shrink-0 items-center gap-1.5">
                      {plugin.shortcut ? (
                        <kbd className="v2-mono rounded-md border border-[var(--v2-border)] bg-[var(--v2-bg)] px-1.5 py-0.5 text-[10px] text-[var(--v2-fg-muted)]">
                          {plugin.shortcut}
                        </kbd>
                      ) : plugin.extensions && plugin.extensions.length > 0 ? (
                        <span className="v2-mono rounded-md border border-[var(--v2-border)] bg-[var(--v2-bg)] px-1.5 py-0.5 text-[9.5px] text-[var(--v2-fg-muted)]">
                          {plugin.extensions[0]}
                        </span>
                      ) : null}
                      {selected ? (
                        <span className="flex h-4 w-4 items-center justify-center rounded-full bg-[var(--v2-accent)] text-white">
                          <Check size={10} strokeWidth={3} />
                        </span>
                      ) : null}
                    </div>
                  </div>
                  <div className="mt-0.5 truncate text-[11px] text-[var(--v2-fg-muted)]">
                    {plugin.subtitle}
                  </div>
                </div>
              </button>
            );
          })}
          {filtered.length === 0 ? (
            <div className="col-span-full rounded-xl border border-dashed border-[var(--v2-border-strong)] py-8 text-center text-[12px] text-[var(--v2-fg-muted)]">
              No comparators match &ldquo;{query}&rdquo;
            </div>
          ) : null}
        </div>

        {/* Optional Left & Right Paths with L/R Badges & Browse Buttons */}
        <div className="grid grid-cols-1 gap-3 border-t border-[var(--v2-border-subtle)] pt-3.5 sm:grid-cols-2">
          <div>
            <span className="mb-1.5 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
              Left {isDirectoryKind ? "Folder" : "Path"} <span className="text-[var(--v2-fg-muted)]">(optional)</span>
            </span>
            <div className="flex items-center gap-1.5">
              <span className="v2-mono inline-flex h-8 w-7 shrink-0 items-center justify-center rounded-lg bg-[var(--v2-surface-subtle)] text-[10.5px] font-semibold text-[var(--v2-fg-muted)]">
                L
              </span>
              <input
                type="text"
                value={draftLeft}
                onChange={(e) => setDraftLeft(e.target.value)}
                placeholder="Leave blank for sample diff"
                className="v2-mono h-8 min-w-0 flex-1 rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] px-3 text-[11.5px] text-[var(--v2-fg)] outline-none placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:bg-[var(--v2-surface)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
              />
              <button
                type="button"
                title={`Browse left ${isDirectoryKind ? "folder" : "file"}`}
                onClick={() => void handleBrowse("left")}
                className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-[var(--v2-border)] bg-[var(--v2-surface)] text-[var(--v2-fg-secondary)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
              >
                <FolderOpen size={14} />
              </button>
            </div>
          </div>
          <div>
            <span className="mb-1.5 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
              Right {isDirectoryKind ? "Folder" : "Path"} <span className="text-[var(--v2-fg-muted)]">(optional)</span>
            </span>
            <div className="flex items-center gap-1.5">
              <span className="v2-mono inline-flex h-8 w-7 shrink-0 items-center justify-center rounded-lg bg-[var(--v2-surface-subtle)] text-[10.5px] font-semibold text-[var(--v2-fg-muted)]">
                R
              </span>
              <input
                type="text"
                value={draftRight}
                onChange={(e) => setDraftRight(e.target.value)}
                placeholder="Leave blank for sample diff"
                className="v2-mono h-8 min-w-0 flex-1 rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] px-3 text-[11.5px] text-[var(--v2-fg)] outline-none placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:bg-[var(--v2-surface)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
              />
              <button
                type="button"
                title={`Browse right ${isDirectoryKind ? "folder" : "file"}`}
                onClick={() => void handleBrowse("right")}
                className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-[var(--v2-border)] bg-[var(--v2-surface)] text-[var(--v2-fg-secondary)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
              >
                <FolderOpen size={14} />
              </button>
            </div>
          </div>
        </div>
      </div>
    </V2Modal>
  );
}
