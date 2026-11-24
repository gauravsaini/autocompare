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
        <input
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
          className="w-24 rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg)] px-[var(--ac-space-2)] py-[var(--ac-space-1)] ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
        />
        <button
          type="button"
          data-testid="mediaview/tolerance-close"
          aria-label="Dismiss tolerance (Esc)"
          title="Dismiss tolerance (Esc)"
          onClick={onClose}
          className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
        >
          <X size={12} aria-hidden="true" />
        </button>
      </div>
    </section>
  );
}
