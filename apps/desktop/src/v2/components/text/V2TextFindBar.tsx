import React from "react";
import { ChevronDown, ChevronRight, Replace, Search, X } from "lucide-react";

export type ReplaceTargetSide = "left" | "right" | "both";

export interface V2TextFindBarProps {
  readonly inputRef: React.RefObject<HTMLInputElement | null>;
  readonly replaceInputRef?: React.RefObject<HTMLInputElement | null>;
  readonly query: string;
  readonly replaceQuery: string;
  readonly replaceOpen: boolean;
  readonly matchCase: boolean;
  readonly targetSide: ReplaceTargetSide;
  readonly hitCount: number;
  readonly activeHit: number;
  readonly onQueryChange: (next: string) => void;
  readonly onReplaceQueryChange: (next: string) => void;
  readonly onToggleReplaceOpen: () => void;
  readonly onToggleMatchCase: () => void;
  readonly onTargetSideChange: (side: ReplaceTargetSide) => void;
  readonly onStep: (delta: 1 | -1) => void;
  readonly onReplaceOne: () => void;
  readonly onReplaceAll: () => void;
  readonly onClose: () => void;
}

export default function V2TextFindBar({
  inputRef,
  replaceInputRef,
  query,
  replaceQuery,
  replaceOpen,
  matchCase,
  targetSide,
  hitCount,
  activeHit,
  onQueryChange,
  onReplaceQueryChange,
  onToggleReplaceOpen,
  onToggleMatchCase,
  onTargetSideChange,
  onStep,
  onReplaceOne,
  onReplaceAll,
  onClose,
}: V2TextFindBarProps): React.JSX.Element {
  return (
    <div className="flex shrink-0 flex-col gap-1.5 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
      {/* Find Row */}
      <div className="flex items-center gap-2">
        <button
          type="button"
          data-testid="v2/toggle-replace"
          title={replaceOpen ? "Hide Replace" : "Show Replace (Cmd+H)"}
          onClick={onToggleReplaceOpen}
          className="inline-flex h-5 w-5 items-center justify-center rounded text-[var(--v2-fg-muted)] hover:bg-[var(--v2-btn-hover)]"
        >
          {replaceOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        </button>
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
          title="Match Case"
          onClick={onToggleMatchCase}
          className={`v2-btn px-2 text-[11px] font-semibold ${
            matchCase ? "border-[var(--v2-accent)] bg-[var(--v2-accent-soft)] text-[var(--v2-accent)]" : ""
          }`}
        >
          Aa
        </button>
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

      {/* Expandable Replace Row (Cmd+H) */}
      {replaceOpen ? (
        <div className="flex items-center gap-2 pl-7">
          <Replace size={13} className="text-[var(--v2-fg-muted)]" />
          <input
            ref={replaceInputRef}
            type="text"
            data-testid="v2/replace-input"
            value={replaceQuery}
            onChange={(e) => onReplaceQueryChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && query.length > 0) {
                onReplaceOne();
              }
            }}
            placeholder="Replace with…"
            className="v2-input v2-mono w-64"
          />
          <select
            aria-label="Replace target side"
            value={targetSide}
            onChange={(e) => onTargetSideChange(e.target.value as ReplaceTargetSide)}
            className="v2-input text-[11.5px]"
          >
            <option value="left">Left side</option>
            <option value="right">Right side</option>
            <option value="both">Both sides</option>
          </select>
          <button
            type="button"
            data-testid="v2/replace-one"
            disabled={query.length === 0 || hitCount === 0}
            onClick={onReplaceOne}
            className="v2-btn"
          >
            Replace
          </button>
          <button
            type="button"
            data-testid="v2/replace-all"
            disabled={query.length === 0}
            onClick={onReplaceAll}
            className="v2-btn v2-btn-primary"
          >
            Replace All
          </button>
        </div>
      ) : null}
    </div>
  );
}
