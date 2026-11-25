/**
 * Wave-233: Media audio tolerance panel migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (mediaview/tolerance-panel, mediaview/tolerance,
 * Wave-C2c: coarse tolerance slider wired to the same value + re-compare
 * path (picture byte-distance 0..255; audio keeps the text box for
 * sub-unit precision, the slider snaps to the rounded value). No
 * geometry change: slider stacks in the existing column flow.
 */
import { useId } from "react";
import { Button, Input, Modal } from "../../../ui";

export interface MediaTolerancePanelProps {
  readonly open?: boolean;
  readonly tolerance: string;
  readonly onToleranceChange: (v: string) => void;
  readonly onRecompare: () => void;
  readonly onClose: () => void;
}

/** Coarse slider domain (picture byte-distance units). */
export const MEDIA_TOLERANCE_SLIDER_MAX = 255;

export default function MediaTolerancePanel(props: MediaTolerancePanelProps): React.JSX.Element | null {
  const { open = true, tolerance, onToleranceChange, onRecompare, onClose } = props;
  const sliderId = useId();
  // Wave-C2c: slider mirrors the text box (rounded + clamped); typing
  // stays the precise path, the slider is the coarse setter.
  const parsed = tolerance.trim() === "" ? 0 : Number(tolerance);
  const sliderValue = Number.isFinite(parsed)
    ? Math.min(Math.max(Math.round(parsed), 0), MEDIA_TOLERANCE_SLIDER_MAX)
    : 0;

  return (
    <Modal
      open={open}
      onClose={onClose}
      label="Audio tolerance"
      testid="mediaview/tolerance-panel"
      closeTestid="mediaview/tolerance-close"
      closeAriaLabel="Dismiss tolerance (Esc)"
    >
      <div className="flex flex-col gap-3 pt-1">
        <label
          htmlFor="mediaview-tolerance"
          className="ac-meta font-normal text-[var(--ac-fg-muted)]"
        >
          Tolerance (decimal threshold for audio diffing)
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
          controlSize="sm"
          className="w-full ac-code"
          autoFocus
        />
        {/* Wave-C2c: coarse slider, same value + same re-compare. */}
        <label
          htmlFor={sliderId}
          className="ac-meta font-normal text-[var(--ac-fg-muted)]"
        >
          Tolerance slider (0–255, coarse)
        </label>
        <input
          id={sliderId}
          type="range"
          data-testid="mediaview/tolerance-slider"
          aria-label="tolerance slider"
          min={0}
          max={MEDIA_TOLERANCE_SLIDER_MAX}
          step={1}
          value={sliderValue}
          onChange={(event) => {
            onToleranceChange(event.target.value);
          }}
        />
        <div className="flex items-center justify-end gap-2">
          <Button
            size="sm"
            variant="secondary"
            onClick={onRecompare}
          >
            Recompare
          </Button>
        </div>
      </div>
    </Modal>
  );
}
