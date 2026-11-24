/**
 * Wave-72: single-row media controls toolbar (Beyond Compare style).
 *
 * Collapses the stacked media chrome (mode line, inline tolerance row)
 * into one toolbar row: the mode indicator (legacy `mediaview/mode`),
 * plus -- in audio mode only -- the tolerance toggle and the recompute
 * button (legacy `mediaview/recompare`). Purely presentational: every
 * value and callback is owned by `MediaView` (existing picture/audio
 * fetch + recompute logic untouched). All legacy testids ride the same
 * controls so the wave-48 spec keeps passing unmodified.
 */

import { SlidersHorizontal } from "lucide-react";
import { Button, ToolbarButton } from "../../../ui";

export interface MediaToolbarProps {
  readonly mode: "picture" | "audio";
  readonly toleranceOpen: boolean;
  readonly onToggleTolerance: () => void;
  readonly comparing: boolean;
  readonly onRecompare: () => void;
}

export default function MediaToolbar(props: MediaToolbarProps): React.JSX.Element {
  const { mode, toleranceOpen, onToggleTolerance, comparing, onRecompare } = props;
  return (
    <div
      role="toolbar"
      aria-label="Media controls"
      data-testid="mediaview/toolbar"
      className="tv-controls-bar"
    >
      <span
        data-testid="mediaview/mode"
        className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {mode === "picture" ? "Picture compare" : "Audio compare"}
      </span>
      {mode === "audio" ? (
        <>
          <span aria-hidden="true" className="tv-controls-sep" />
          <ToolbarButton
            data-testid="mediaview/toolbar-tolerance"
            label={toleranceOpen ? "Hide tolerance (Esc)" : "Show tolerance"}
            title="Audio tolerance"
            expanded={toleranceOpen}
            onClick={onToggleTolerance}
          >
            <SlidersHorizontal size={14} strokeWidth={1.75} aria-hidden="true" />
            <span>Tolerance</span>
          </ToolbarButton>
          <Button
            data-testid="mediaview/recompare"
            onClick={onRecompare}
            disabled={comparing}
          >
            Recompare
          </Button>
        </>
      ) : null}
    </div>
  );
}
