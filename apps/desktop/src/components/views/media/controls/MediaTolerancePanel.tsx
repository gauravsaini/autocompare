/**
 * Wave-72: collapsible audio tolerance panel (Beyond Compare style).
 *
 * In-flow collapsible (never `position: absolute`, so the wave-7
 * overlap gate holds with the panel open): open by default in audio
 * mode for backward compat (the wave-48 spec fills the tolerance box
 * on load), toggled from the toolbar or Esc. Purely presentational:
 * tolerance state and the recompute activation live in `MediaView`
 * (wave-48 logic byte-identical). The legacy `mediaview/tolerance`
 * testid rides the same input.
 */
import { X } from "lucide-react";
import { Input, ToolbarButton } from "../../../ui";

export interface MediaTolerancePanelProps {
  readonly tolerance: string;
  readonly onToleranceChange: (v: string) => void;
  readonly onRecompare: () => void;
  readonly onClose: () => void;
}

export default function MediaTolerancePanel(props: MediaTolerancePanelProps): React.JSX.Element {
  const { tolerance, onToleranceChange, onRecompare, onClose } = props;
  return (
    <section data-testid="mediaview/tolerance-panel" className="tv-controls-panel">
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="audio tolerance">
        <label
          htmlFor="mediaview-tolerance"
          className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          Tolerance
        </label>
        <Input
          id="mediaview-tolerance"
          data-testid="mediaview/tolerance"
          value={tolerance}
          onChange={(event) => {
            onToleranceChange(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") onRecompare();
            else if (event.key === "Escape") onClose();
          }}
          inputMode="decimal"
          placeholder="exact"
          autoComplete="off"
          spellCheck={false}
          className="w-24 ac-mono"
        />
        <ToolbarButton
          data-testid="mediaview/tolerance-close"
          label="Dismiss tolerance (Esc)"
          onClick={onClose}
          icon={X}
        />
      </div>
    </section>
  );
}
