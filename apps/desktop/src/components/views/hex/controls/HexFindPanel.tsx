/**
 * Wave-71: collapsible hex find panel (Beyond Compare Ctrl+F style).
 *
 * In-flow collapsible (never `position: absolute`, so the wave-7
 * overlap gate holds with the panel open): open by default for backward
 * compat (wave-19/62 specs fill the find box on load), toggled from the
 * toolbar, Ctrl+F, or Esc. Purely presentational: query state, match
 * math and activation live in `HexView` (wave-19 logic byte-identical).
 * All legacy `hex-find-*` testids ride the same inputs.
 */
import { X } from "lucide-react";
import { Button, Input, ToolbarButton } from "../../../ui";

export interface HexFindPanelProps {
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly finding: boolean;
  readonly onFindNext: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function HexFindPanel(props: HexFindPanelProps): React.JSX.Element {
  const { query, onQueryChange, finding, onFindNext, onClose, statusText } = props;
  return (
    <section data-testid="hex/find-panel" className="tv-controls-panel">
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="find bytes">
        <label
          htmlFor="hex-find-box"
          className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          Find hex
        </label>
        <Input
          id="hex-find-box"
          data-testid="hex-find-input"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") onFindNext();
            else if (event.key === "Escape") onClose();
          }}
          placeholder="de ad be ef"
          autoComplete="off"
          spellCheck={false}
          className="ac-mono min-w-48 flex-1 text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)]"
        />
        <Button
          data-testid="hex-find-next"
          onClick={onFindNext}
          disabled={finding}
        >
          {finding ? "Finding…" : "Find next"}
        </Button>
        <ToolbarButton
          data-testid="hex-find-close"
          label="Dismiss find (Esc)"
          title="Dismiss find (Esc)"
          onClick={onClose}
        >
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </ToolbarButton>
      </div>
      <p
        data-testid="hex-find-status"
        className="pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {statusText}
      </p>
    </section>
  );
}
