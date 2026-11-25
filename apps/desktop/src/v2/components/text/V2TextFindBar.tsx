import React from "react";
import { Search, X } from "lucide-react";

export interface V2TextFindBarProps {
  readonly inputRef: React.RefObject<HTMLInputElement | null>;
  readonly query: string;
  readonly hitCount: number;
  readonly activeHit: number;
  readonly onQueryChange: (next: string) => void;
  readonly onStep: (delta: 1 | -1) => void;
  readonly onClose: () => void;
}

export default function V2TextFindBar({
  inputRef,
  query,
  hitCount,
  activeHit,
  onQueryChange,
  onStep,
  onClose,
}: V2TextFindBarProps): React.JSX.Element {
  return (
    <div className="flex shrink-0 items-center gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
      <Search size={13} className="text-[var(--v2-fg-muted)]" />
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && hitCount > 0) {
            onStep(e.shiftKey ? -1 : 1);
          }
        }}
        placeholder="Find in diff…"
        className="v2-input v2-mono w-64"
      />
      <button
        type="button"
        disabled={hitCount === 0}
        onClick={() => onStep(-1)}
        className="v2-btn"
      >
        Prev
      </button>
      <button
        type="button"
        disabled={hitCount === 0}
        onClick={() => onStep(1)}
        className="v2-btn"
      >
        Next
      </button>
      <span className="text-[11px] text-[var(--v2-fg-muted)]">
        {query.trim()
          ? hitCount === 0
            ? "No matches"
            : `${activeHit + 1} of ${hitCount}`
          : "Press Enter to jump"}
      </span>
      <button
        type="button"
        aria-label="Close find"
        onClick={onClose}
        className="ml-auto inline-flex h-5 w-5 items-center justify-center rounded text-[var(--v2-fg-muted)] hover:bg-[var(--v2-btn-hover)]"
      >
        <X size={13} />
      </button>
    </div>
  );
}
