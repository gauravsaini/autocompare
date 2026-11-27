import React from "react";
import { RefreshCw, X } from "lucide-react";

export interface V2ReloadPromptBannerProps {
  readonly changedPaths: readonly string[];
  readonly onReload: () => void;
  readonly onDismiss: () => void;
}

export default function V2ReloadPromptBanner({
  changedPaths,
  onReload,
  onDismiss,
}: V2ReloadPromptBannerProps): React.JSX.Element | null {
  if (changedPaths.length === 0) return null;

  const labels = changedPaths.map((p) => p.split("/").pop() || p);
  const summary =
    labels.length === 1
      ? `${labels[0]} changed on disk.`
      : `${labels.join(" and ")} changed on disk.`;

  return (
    <div
      role="status"
      data-testid="v2/reload-banner"
      className="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chg-bg)] px-3 py-1.5 text-[12px] text-[var(--v2-chg-fg)]"
    >
      <div className="flex min-w-0 items-center gap-2">
        <RefreshCw size={13} className="shrink-0" />
        <span className="truncate font-medium">
          {summary} Reload comparison to see the latest changes?
        </span>
      </div>
      <div className="flex shrink-0 items-center gap-1.5">
        <button
          type="button"
          data-testid="v2/reload-banner-confirm"
          onClick={onReload}
          className="v2-btn v2-btn-primary h-[21px] px-2.5 text-[11px]"
        >
          Reload
        </button>
        <button
          type="button"
          data-testid="v2/reload-banner-dismiss"
          onClick={onDismiss}
          className="v2-btn h-[21px] px-2 text-[11px]"
        >
          Ignore
        </button>
        <button
          type="button"
          aria-label="Dismiss reload prompt"
          onClick={onDismiss}
          className="inline-flex h-5 w-5 items-center justify-center rounded hover:bg-[var(--v2-btn-hover)]"
        >
          <X size={12} />
        </button>
      </div>
    </div>
  );
}
