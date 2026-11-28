import React from "react";
import { FolderOpen, Save } from "lucide-react";
import { browsePath } from "../utils/dialog";

export interface V2DualPathBarProps {
  readonly leftValue: string;
  readonly rightValue: string;
  readonly onLeftChange: (value: string) => void;
  readonly onRightChange: (value: string) => void;
  readonly onSubmit: (left: string, right: string) => void;
  readonly leftPlaceholder?: string;
  readonly rightPlaceholder?: string;
  readonly directory?: boolean;
  readonly splitPct?: number;
  readonly onSaveSide?: (side: "left" | "right") => void;
  readonly onBrowseUnavailable?: () => void;
  readonly leftMeta?: React.ReactNode;
  readonly rightMeta?: React.ReactNode;
}

/**
 * Shared dual-pane path header for all 2-side AutoCompare v2 views.
 * Stays lockstep-aligned with `useSplitPane` when `splitPct` is passed.
 */
export default function V2DualPathBar({
  leftValue,
  rightValue,
  onLeftChange,
  onRightChange,
  onSubmit,
  leftPlaceholder = "Left path…",
  rightPlaceholder = "Right path…",
  directory = false,
  splitPct = 50,
  onSaveSide,
  onBrowseUnavailable,
  leftMeta,
  rightMeta,
}: V2DualPathBarProps): React.JSX.Element {
  const handleBrowse = async (side: "left" | "right"): Promise<void> => {
    const picked = await browsePath(directory);
    if (!picked) {
      if (
        typeof window !== "undefined" &&
        !("__TAURI_INTERNALS__" in window) &&
        !("__TAURI__" in window)
      ) {
        onBrowseUnavailable?.();
      }
      return;
    }
    const nextL = side === "left" ? picked : leftValue;
    const nextR = side === "right" ? picked : rightValue;
    if (side === "left") onLeftChange(picked);
    else onRightChange(picked);
    onSubmit(nextL.trim(), nextR.trim());
  };

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit(leftValue.trim(), rightValue.trim());
      }}
      style={{ gridTemplateColumns: `${splitPct}fr ${100 - splitPct}fr` }}
      className="grid shrink-0 border-b border-[var(--v2-border)] bg-[var(--v2-surface)]"
    >
      {(["left", "right"] as const).map((side) => {
        const val = side === "left" ? leftValue : rightValue;
        const setVal = side === "left" ? onLeftChange : onRightChange;
        const placeholder = side === "left" ? leftPlaceholder : rightPlaceholder;
        const meta = side === "left" ? leftMeta : rightMeta;
        return (
          <div
            key={side}
            className={`flex flex-col gap-1.5 px-3 py-2 ${
              side === "left" ? "border-r border-[var(--v2-border)]" : ""
            }`}
          >
            <div className="flex items-center gap-1.5">
              <span
                aria-hidden="true"
                className="v2-mono inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-lg bg-[var(--v2-surface-subtle)] text-[10.5px] font-semibold uppercase text-[var(--v2-fg-muted)]"
              >
                {side === "left" ? "L" : "R"}
              </span>
              <input
                type="text"
                data-testid={`v2/path-${side}`}
                value={val}
                onChange={(e) => setVal(e.target.value)}
                placeholder={placeholder}
                className="v2-mono h-7 min-w-0 flex-1 rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)] px-2.5 text-[11.5px] text-[var(--v2-fg)] outline-none transition-colors placeholder:text-[var(--v2-fg-muted)] focus:border-[var(--v2-accent)] focus:bg-[var(--v2-surface)] focus:ring-2 focus:ring-[var(--v2-accent-ring)]"
              />
              <button
                type="button"
                title={`Browse ${side} ${directory ? "folder" : "file"}`}
                onClick={() => void handleBrowse(side)}
                className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-[var(--v2-border)] bg-[var(--v2-surface)] text-[var(--v2-fg-secondary)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
              >
                <FolderOpen size={13.5} />
              </button>
              {onSaveSide ? (
                <button
                  type="button"
                  title={`Save ${side} file`}
                  onClick={() => onSaveSide(side)}
                  className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-[var(--v2-border)] bg-[var(--v2-surface)] text-[var(--v2-fg-secondary)] shadow-sm transition-colors hover:border-[var(--v2-border-strong)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                >
                  <Save size={13.5} />
                </button>
              ) : null}
            </div>
            {meta ? (
              <div className="v2-mono flex items-center gap-3 pl-7 text-[10.5px] text-[var(--v2-fg-muted)]">
                {meta}
              </div>
            ) : null}
          </div>
        );
      })}
      <button type="submit" className="sr-only" tabIndex={-1}>
        Compare paths
      </button>
    </form>
  );
}
