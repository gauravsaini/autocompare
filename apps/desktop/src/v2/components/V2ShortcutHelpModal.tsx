import React, { useMemo, useState } from "react";
import { Search } from "lucide-react";
import V2Modal from "./V2Modal";

export interface V2ShortcutHelpModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
}

interface ShortcutEntry {
  readonly keys: readonly string[];
  readonly action: string;
}

interface ShortcutGroup {
  readonly category: string;
  readonly items: readonly ShortcutEntry[];
}

const SHORTCUT_GROUPS: readonly ShortcutGroup[] = [
  {
    category: "Global & Sessions",
    items: [
      { keys: ["⌘", "K"], action: "Open Command Palette" },
      { keys: ["⌘", "/"], action: "Open Keyboard Shortcut Cheat Sheet (or press ?)" },
      { keys: ["⌘", "N"], action: "New Comparison Session" },
      { keys: ["⌘", "S"], action: "Save Active Workspace Session" },
      { keys: ["⌘", "W"], action: "Close Active Tab" },
      { keys: ["⌘", ","], action: "Open Preferences" },
      { keys: ["⌘", "1–7"], action: "Quick-open Text, Folder, Hex, Table, Media, Sync, Merge" },
    ],
  },
  {
    category: "Text Compare",
    items: [
      { keys: ["N", "/", "P"], action: "Jump to Next / Previous Difference Section" },
      { keys: ["⌘", "F"], action: "Find in Text Compare" },
      { keys: ["⌘", "H"], action: "Find & Replace in Active Side" },
      { keys: ["⌘", "D"], action: "Copy Selected Diff Section to Right" },
      { keys: ["⌘", "E"], action: "Copy Selected Diff Section to Left" },
      { keys: ["F7"], action: "Align Selected Line With… (Manual Alignment)" },
      { keys: ["Shift", "Click"], action: "Select Multi-Line Range for Bulk Copy" },
      { keys: ["⌘", "Z"], action: "Undo / Shift+⌘+Z Redo Text Edits" },
    ],
  },
  {
    category: "3-Way Text Merge",
    items: [
      { keys: ["Alt", "1"], action: "Resolve Active Conflict → Take Left" },
      { keys: ["Alt", "2"], action: "Resolve Active Conflict → Take Base" },
      { keys: ["Alt", "3"], action: "Resolve Active Conflict → Take Right" },
      { keys: ["Alt", "4"], action: "Resolve Active Conflict → Take Both (L → R)" },
      { keys: ["Alt", "N"], action: "Jump to Next Conflict" },
      { keys: ["Alt", "P"], action: "Jump to Previous Conflict" },
    ],
  },
  {
    category: "Folder, Table & Hex",
    items: [
      { keys: ["Right-Click"], action: "Folder Compare: Copy to Right/Left, Delete, Open Text Diff" },
      { keys: ["Double-Click"], action: "Table Compare: Inline Edit Cell (Enter saves, Esc cancels)" },
      { keys: ["Double-Click"], action: "Hex Compare: Inline Edit Byte (⌘Z undo)" },
      { keys: ["Click"], action: "Hex Compare: Select Byte for LE/BE Data Inspector" },
      { keys: ["⌘", "F"], action: "Table & Hex Compare: Open Docked Search Bar" },
    ],
  },
];

export default function V2ShortcutHelpModal({
  open,
  onClose,
}: V2ShortcutHelpModalProps): React.JSX.Element {
  const [query, setQuery] = useState("");

  const filteredGroups = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return SHORTCUT_GROUPS;
    return SHORTCUT_GROUPS.map((group) => ({
      ...group,
      items: group.items.filter(
        (item) =>
          group.category.toLowerCase().includes(q) ||
          item.action.toLowerCase().includes(q) ||
          item.keys.join(" ").toLowerCase().includes(q),
      ),
    })).filter((group) => group.items.length > 0);
  }, [query]);

  return (
    <V2Modal
      open={open}
      title="Keyboard Shortcuts"
      subtitle="Press ? or ⌘/ anytime to open this cheat sheet"
      widthClass="max-w-xl"
      onClose={onClose}
      footer={
        <button type="button" onClick={onClose} className="v2-btn v2-btn-primary">
          Done
        </button>
      }
    >
      <div data-testid="v2/shortcuts-modal" className="space-y-3.5 text-[12px]">
        <div className="relative flex items-center">
          <Search
            size={14}
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--v2-fg-muted)]"
          />
          <input
            autoFocus
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Filter shortcuts (e.g. merge, find, align, hex)…"
            className="h-9 w-full rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] pl-9 pr-3 text-[12.5px] text-[var(--v2-fg)] outline-none transition-colors placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:bg-[var(--v2-surface)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
          />
        </div>

        <div
          tabIndex={0}
          aria-label="Keyboard shortcuts list"
          className="max-h-[380px] space-y-3 overflow-y-auto pr-1 outline-none"
        >
          {filteredGroups.length === 0 ? (
            <div className="rounded-xl border border-dashed border-[var(--v2-border-strong)] py-8 text-center text-[var(--v2-fg-muted)]">
              No shortcuts match &ldquo;{query}&rdquo;.
            </div>
          ) : (
            filteredGroups.map((group) => (
              <div key={group.category}>
                <h3 className="mb-1.5 text-[10.5px] font-semibold uppercase tracking-wide text-[var(--v2-fg-muted)]">
                  {group.category}
                </h3>
                <div className="divide-y divide-[var(--v2-border-subtle)] rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)]/60">
                  {group.items.map((item) => (
                    <div
                      key={`${group.category}-${item.action}`}
                      className="flex items-center justify-between gap-3 px-3 py-1.5"
                    >
                      <span className="text-[var(--v2-fg)]">{item.action}</span>
                      <span className="flex shrink-0 items-center gap-1">
                        {item.keys.map((k, idx) => (
                          <kbd
                            key={idx}
                            className="v2-mono rounded-[4px] border border-[var(--v2-border)] bg-[var(--v2-surface)] px-1.5 py-0.5 text-[10.5px] font-medium text-[var(--v2-fg-secondary)] shadow-[0_1px_0_rgba(0,0,0,0.06)]"
                          >
                            {k}
                          </kbd>
                        ))}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </V2Modal>
  );
}
