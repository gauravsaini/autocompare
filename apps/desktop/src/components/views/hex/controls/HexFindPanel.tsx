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
import { Button, Field, Input, StatusText, ToolbarButton } from "../../../ui";

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
      <div className="ac-control-group" role="search" aria-label="find bytes">
        <Field label="Find hex" htmlFor="hex-find-box" className="min-w-0 flex-1">
          <Input
            id="hex-find-box"
            data-testid="hex-find-input"
            controlSize="sm"
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") onFindNext();
              else if (event.key === "Escape") onClose();
            }}
            placeholder="de ad be ef"
            autoComplete="off"
            spellCheck={false}
            className="ac-code min-w-48 flex-1"
          />
        </Field>
        <Button
          data-testid="hex-find-next"
          size="sm"
          onClick={onFindNext}
          disabled={finding}
        >
          {finding ? "Finding…" : "Find next"}
        </Button>
        <ToolbarButton
          data-testid="hex-find-close"
          label="Dismiss find (Esc)"
          title="Dismiss find (Esc)"
          icon={X}
          onClick={onClose}
        />
      </div>
      <StatusText
        data-testid="hex-find-status"
        role="status"
        className="block pt-[var(--ac-space-1)]"
      >
        {statusText}
      </StatusText>
    </section>
  );
}
