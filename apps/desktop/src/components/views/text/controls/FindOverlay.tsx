/**
 * Wave-66R: collapsible find bar (Beyond Compare style Ctrl+F overlay).
 *
 * In-flow collapsible (never `position: absolute`, so the wave-7
 * overlap gate in textfind-61 / findshortcut-62 keeps passing):
 * open by default for backward compat, toggled from the toolbar,
 * Ctrl+F, or Esc. Purely presentational: query state, hit math and
 * activation live in `TextView` (wave-61 logic byte-identical).
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
    <section data-testid="textview/find-panel" className="tv-controls-panel">
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
        <Button data-testid="textview/find-prev" onClick={onPrev} disabled={prevDisabled}>
          Prev
        </Button>
        <Button data-testid="textview/find-next" onClick={onNext} disabled={nextDisabled}>
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
