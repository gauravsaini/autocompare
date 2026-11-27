import React, { useMemo, useState } from "react";
import { Check, Search } from "lucide-react";
import V2Modal from "./V2Modal";
import { listComparators } from "../plugins/registry";
import type { SessionKind } from "../types";

export interface V2NewSessionModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onCreate: (kind: SessionKind, left: string, right: string) => void;
}

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

  const handleOpen = (kind: SessionKind): void => {
    onCreate(kind, draftLeft.trim(), draftRight.trim());
    setDraftLeft("");
    setDraftRight("");
    setQuery("");
    onClose();
  };

  const selectedPlugin =
    comparators.find((p) => p.id === draftKind) ?? comparators[0];

  return (
    <V2Modal
      open={open}
      title="New Comparison Session"
      subtitle="Select a comparator card (double-click to open immediately) or enter optional paths below"
      widthClass="max-w-2xl"
      onClose={onClose}
      footer={
        <div className="flex w-full items-center justify-between gap-2">
          <span className="text-[11px] text-[var(--v2-fg-muted)]">
            {selectedPlugin ? `Selected: ${selectedPlugin.title}` : "Select a comparator"}
          </span>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className="v2-btn">
              Cancel
            </button>
            <button
              type="button"
              data-testid="v2/new-session-open-btn"
              onClick={() => handleOpen(draftKind)}
              className="v2-btn v2-btn-primary"
            >
              Open Comparison
            </button>
          </div>
        </div>
      }
    >
      <div className="space-y-3 text-[12px]">
        {/* Search / Filter Bar */}
        <div className="relative">
          <Search
            size={13}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
          />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Filter comparators (e.g. text, folder, json, toml, merge, hex)…"
            className="v2-input w-full pl-8 text-[12px]"
          />
        </div>

        {/* Responsive Multi-Column Comparator Grid */}
        <div
          data-testid="v2/new-session-grid"
          className="grid max-h-[290px] grid-cols-2 gap-2 overflow-y-auto pr-0.5 sm:grid-cols-3"
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
                className={`group relative flex flex-col items-start gap-1.5 rounded-[9px] border p-2.5 text-left transition ${
                  selected
                    ? "border-[var(--v2-accent)] bg-[var(--v2-accent-soft)]/50 ring-1 ring-[var(--v2-accent)]"
                    : "border-[var(--v2-border)] bg-[var(--v2-surface)] hover:border-[var(--v2-fg-muted)]/60 hover:bg-[var(--v2-surface-subtle)]"
                }`}
              >
                <div className="flex w-full items-center justify-between gap-2">
                  <span
                    className="flex h-7 w-7 shrink-0 items-center justify-center rounded-[7px]"
                    style={{
                      backgroundColor: `${plugin.accent}1a`,
                      color: plugin.accent,
                    }}
                  >
                    <Icon size={15} strokeWidth={2} />
                  </span>
                  <div className="flex items-center gap-1">
                    {plugin.shortcut ? (
                      <kbd className="rounded border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--v2-fg-muted)]">
                        {plugin.shortcut}
                      </kbd>
                    ) : plugin.extensions && plugin.extensions.length > 0 ? (
                      <span className="rounded bg-[var(--v2-surface-subtle)] px-1.5 py-0.5 font-mono text-[9.5px] text-[var(--v2-fg-muted)]">
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
                <div className="min-w-0 w-full">
                  <div className="truncate text-[12px] font-semibold text-[var(--v2-fg)]">
                    {plugin.title}
                  </div>
                  <div className="line-clamp-2 text-[10.5px] leading-[14px] text-[var(--v2-fg-muted)]">
                    {plugin.subtitle}
                  </div>
                </div>
              </button>
            );
          })}
          {filtered.length === 0 ? (
            <div className="col-span-full py-6 text-center text-[11.5px] text-[var(--v2-fg-muted)]">
              No comparators match &ldquo;{query}&rdquo;
            </div>
          ) : null}
        </div>

        {/* Optional Left & Right Paths in a Compact 2-Column Row */}
        <div className="grid grid-cols-1 gap-2 border-t border-[var(--v2-border)] pt-2.5 sm:grid-cols-2">
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Left Path (optional)
            </span>
            <input
              type="text"
              value={draftLeft}
              onChange={(e) => setDraftLeft(e.target.value)}
              placeholder="Leave blank for sample diff"
              className="v2-input v2-mono w-full"
            />
          </div>
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Right Path (optional)
            </span>
            <input
              type="text"
              value={draftRight}
              onChange={(e) => setDraftRight(e.target.value)}
              placeholder="Leave blank for sample diff"
              className="v2-input v2-mono w-full"
            />
          </div>
        </div>
      </div>
    </V2Modal>
  );
}
