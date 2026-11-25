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
import { Button, StatusText, ToolbarButton } from "../../../ui";

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
      <StatusText
        data-testid="mediaview/mode"
        className="ac-chrome-label font-normal text-[var(--ac-fg-muted)]"
      >
        {mode === "picture" ? "Picture compare" : "Audio compare"}
      </StatusText>
      {mode === "audio" ? (
        <>
          <span aria-hidden="true" className="tv-controls-sep" />
          <ToolbarButton
            data-testid="mediaview/toolbar-tolerance"
            label={toleranceOpen ? "Hide tolerance (Esc)" : "Show tolerance"}
            title="Audio tolerance"
            icon={SlidersHorizontal}
            size="sm"
            expanded={toleranceOpen}
            onClick={onToggleTolerance}
          >
            <span>Tolerance</span>
          </ToolbarButton>
          <Button
            data-testid="mediaview/recompare"
            size="sm"
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
