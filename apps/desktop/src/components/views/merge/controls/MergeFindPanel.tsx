/**
 * Wave-70: collapsible merge find panel (Beyond Compare Ctrl+F style).
 *
 * In-flow collapsible (never `position: absolute`, so the wave-7
 * overlap gate holds with the panel open): open by default for backward
 * compat, toggled from the toolbar, Ctrl+F, or Esc. Purely
 * presentational: query state, hit math and activation live in
 * `MergeView` (wave-66F logic byte-identical). All legacy
 * `merge/find-*` testids ride the same inputs.
 */
import { X } from "lucide-react";
import { Button, Input, ToolbarButton } from "../../../ui";

export interface MergeFindPanelProps {
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly prevDisabled: boolean;
  readonly nextDisabled: boolean;
  readonly onPrev: () => void;
  readonly onNext: () => void;
  readonly onRun: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function MergeFindPanel(props: MergeFindPanelProps): React.JSX.Element {
  const { query, onQueryChange, prevDisabled, nextDisabled, onPrev, onNext, onRun, onClose, statusText } =
    props;
  return (
    <section data-testid="merge/find-panel" className="tv-controls-panel">
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="find in merge">
        <label
          htmlFor="merge-find-box"
          className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          Find
        </label>
        <Input
          id="merge-find-box"
          data-testid="merge/find-input"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") onRun();
            else if (event.key === "Escape") onClose();
          }}
          placeholder="text to find"
          autoComplete="off"
          spellCheck={false}
          className="w-40 rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
        />
        <Button
          data-testid="merge/find-prev"
          onClick={onPrev}
          disabled={prevDisabled}
          className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45"
        >
          Prev
        </Button>
        <Button
          data-testid="merge/find-next"
          onClick={onNext}
          disabled={nextDisabled}
          className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45"
        >
          Next
        </Button>
        <ToolbarButton
          data-testid="merge/find-close"
          label="Dismiss find (Esc)"
          aria-label="Dismiss find (Esc)"
          title="Dismiss find (Esc)"
          onClick={onClose}
        >
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </ToolbarButton>
      </div>
      <p
        data-testid="merge/find-status"
        className="pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {statusText}
      </p>
    </section>
  );
}
