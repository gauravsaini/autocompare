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
      className="grid shrink-0 border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
    >
      {(["left", "right"] as const).map((side) => {
        const val = side === "left" ? leftValue : rightValue;
        const setVal = side === "left" ? onLeftChange : onRightChange;
        const placeholder = side === "left" ? leftPlaceholder : rightPlaceholder;
        const meta = side === "left" ? leftMeta : rightMeta;
        return (
          <div
            key={side}
            className={`flex flex-col gap-1 px-2.5 py-1.5 ${
              side === "left" ? "border-r border-[var(--v2-border)]" : ""
            }`}
          >
            <div className="flex items-center gap-1.5">
              <input
                type="text"
                data-testid={`v2/path-${side}`}
                value={val}
                onChange={(e) => setVal(e.target.value)}
                placeholder={placeholder}
                className="v2-input v2-mono min-w-0 flex-1 text-[11.5px]"
              />
              <button
                type="button"
                title={`Browse ${side} ${directory ? "folder" : "file"}`}
                onClick={() => void handleBrowse(side)}
                className="v2-btn px-2"
              >
                <FolderOpen size={13} />
              </button>
              {onSaveSide ? (
                <button
                  type="button"
                  title={`Save ${side} file`}
                  onClick={() => onSaveSide(side)}
                  className="v2-btn px-2"
                >
                  <Save size={13} />
                </button>
              ) : null}
            </div>
            {meta ? (
              <div className="flex items-center gap-3 text-[11px] text-[var(--v2-fg-muted)]">
                {meta}
              </div>
            ) : null}
          </div>
        );
      })}
    </form>
  );
}
