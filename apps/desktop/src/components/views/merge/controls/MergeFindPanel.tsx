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
      <div className="ac-control-group" role="search" aria-label="find in merge">
        <label
          htmlFor="merge-find-box"
          className="ac-meta font-normal text-[var(--ac-fg-muted)]"
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
          className="ac-code w-40"
        />
        <Button
          data-testid="merge/find-prev"
          aria-label="Previous find hit"
          onClick={onPrev}
          disabled={prevDisabled}
        >
          Prev
        </Button>
        <Button
          data-testid="merge/find-next"
          aria-label="Next find hit"
          onClick={onNext}
          disabled={nextDisabled}
        >
          Next
        </Button>
        <ToolbarButton
          data-testid="merge/find-close"
          label="Dismiss find (Esc)"
          title="Dismiss find (Esc)"
          icon={X}
          onClick={onClose}
        />
      </div>
      <p
        data-testid="merge/find-status"
        className="ac-meta pt-[var(--ac-space-1)] font-normal text-[var(--ac-fg-muted)]"
      >
        {statusText}
      </p>
    </section>
  );
}
