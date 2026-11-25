/**
 * Wave-233: Media audio tolerance panel migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (mediaview/tolerance-panel, mediaview/tolerance,
 * mediaview/tolerance-close).
 */
import { Button, Input, Modal } from "../../../ui";

export interface MediaTolerancePanelProps {
  readonly open?: boolean;
  readonly tolerance: string;
  readonly onToleranceChange: (v: string) => void;
  readonly onRecompare: () => void;
  readonly onClose: () => void;
}

export default function MediaTolerancePanel(props: MediaTolerancePanelProps): React.JSX.Element | null {
  const { open = true, tolerance, onToleranceChange, onRecompare, onClose } = props;

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
