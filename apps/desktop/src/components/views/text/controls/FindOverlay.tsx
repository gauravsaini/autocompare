/**
 * Wave-199c: find rides the chrome overlay stack (Beyond Compare
 * style Ctrl+F popover).
 *
 * Absolute popover (never in-flow): the `tv-chrome` wrapper in
 * `TextView` is relative and hosts an absolute overlay column, so an
 * open find panel floats over the rows instead of pushing
 * `textview/row-0` down (firstRowTop<=140 holds open AND closed).
 * Purely presentational: query state, hit math and activation live
 * in `TextView` (wave-61 logic byte-identical). All testids/aria
 * unchanged.
 */
import { X } from "lucide-react";
import { Button, Input, ToolbarButton } from "../../../ui";

export interface FindOverlayProps {
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

export default function FindOverlay(props: FindOverlayProps): React.JSX.Element {
  const { query, onQueryChange, prevDisabled, nextDisabled, onPrev, onNext, onRun, onClose, statusText } = props;
  return (
    <section
      data-testid="textview/find-panel"
      className="tv-controls-panel rounded-md border border-[var(--ac-border-subtle)] shadow-lg"
    >
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="find in text">
        <label htmlFor="textview-find-box" className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          Find
        </label>
        <Input
          id="textview-find-box"
          data-testid="textview/find-input"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") onRun();
            else if (event.key === "Escape") onClose();
          }}
          placeholder="text to find"
          autoComplete="off"
          spellCheck={false}
          className="ac-mono w-40 text-[length:var(--ac-font-size-code)]"
        />
        <Button data-testid="textview/find-prev" aria-label="Previous find hit" onClick={onPrev} disabled={prevDisabled}>
          Prev
        </Button>
        <Button data-testid="textview/find-next" aria-label="Next find hit" onClick={onNext} disabled={nextDisabled}>
          Next
        </Button>
        <ToolbarButton data-testid="textview/find-close" label="Dismiss find (Esc)" title="Dismiss find (Esc)" onClick={onClose}>
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </ToolbarButton>
      </div>
      <p data-testid="textview/find-status" className="pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
        {statusText}
      </p>
    </section>
  );
}
