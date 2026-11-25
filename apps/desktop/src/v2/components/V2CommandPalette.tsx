import React, { useEffect, useMemo, useRef, useState } from "react";
import { Search } from "lucide-react";

export interface PaletteCommand {
  readonly id: string;
  readonly label: string;
  readonly category: string;
  readonly shortcut?: string;
  readonly run: () => void;
}

export interface V2CommandPaletteProps {
  readonly open: boolean;
  readonly commands: readonly PaletteCommand[];
  readonly onClose: () => void;
}

/**
 * Raycast / Zed-style Cmd+K Command Palette for AutoCompare v2.
 */
export default function V2CommandPalette({
  open,
  commands,
  onClose,
}: V2CommandPaletteProps): React.JSX.Element | null {
  const [query, setQuery] = useState("");
  const [idx, setIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setIdx(0);
      setTimeout(() => inputRef.current?.focus(), 10);
    }
  }, [open]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(
      (c) => c.label.toLowerCase().includes(q) || c.category.toLowerCase().includes(q),
    );
  }, [commands, query]);

  useEffect(() => {
    setIdx(0);
  }, [query]);

  if (!open) return null;

  const execute = (cmd: PaletteCommand | undefined): void => {
    if (!cmd) return;
    onClose();
    cmd.run();
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/30 pt-[14vh] backdrop-blur-[1px]"
      onClick={onClose}
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-[10px] border border-[var(--v2-border)] bg-[var(--v2-surface)] shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-[var(--v2-border-subtle)] px-3.5 py-2.5">
          <Search size={14} className="text-[var(--v2-fg-muted)]" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                onClose();
              } else if (e.key === "ArrowDown") {
                e.preventDefault();
                setIdx((i) => (filtered.length > 0 ? (i + 1) % filtered.length : 0));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setIdx((i) => (filtered.length > 0 ? (i - 1 + filtered.length) % filtered.length : 0));
              } else if (e.key === "Enter") {
                e.preventDefault();
                execute(filtered[idx]);
              }
            }}
            placeholder="Type a command or comparison mode…"
            className="w-full bg-transparent text-[13px] text-[var(--v2-fg)] outline-none placeholder:text-[var(--v2-fg-muted)]"
          />
          <kbd className="rounded border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-1.5 py-0.5 text-[10px] text-[var(--v2-fg-muted)]">
            Esc
          </kbd>
        </div>

        <div className="max-h-[300px] overflow-y-auto p-1.5">
          {filtered.length === 0 ? (
            <div className="px-3 py-6 text-center text-[12px] text-[var(--v2-fg-muted)]">
              No matching commands
            </div>
          ) : (
            filtered.map((cmd, i) => {
              const selected = i === idx;
              return (
                <button
                  key={cmd.id}
                  type="button"
                  onMouseEnter={() => setIdx(i)}
                  onClick={() => execute(cmd)}
                  className={`flex w-full items-center justify-between rounded-[6px] px-3 py-1.5 text-left text-[12px] ${
                    selected
                      ? "bg-[var(--v2-accent)] text-white"
                      : "text-[var(--v2-fg)] hover:bg-[var(--v2-btn-hover)]"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className={`text-[10.5px] ${selected ? "text-white/80" : "text-[var(--v2-fg-muted)]"}`}>
                      {cmd.category}
                    </span>
                    <span className="font-medium">{cmd.label}</span>
                  </div>
                  {cmd.shortcut ? (
                    <kbd
                      className={`v2-mono rounded px-1.5 py-0.5 text-[10px] ${
                        selected
                          ? "bg-white/20 text-white"
                          : "border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] text-[var(--v2-fg-muted)]"
                      }`}
                    >
                      {cmd.shortcut}
                    </kbd>
                  ) : null}
                </button>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
