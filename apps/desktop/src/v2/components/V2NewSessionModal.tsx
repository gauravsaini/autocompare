import React, { useState } from "react";
import V2Modal from "./V2Modal";
import { listComparators } from "../plugins/registry";
import type { SessionKind } from "../types";

export interface V2NewSessionModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onCreate: (kind: SessionKind, left: string, right: string) => void;
}

export default function V2NewSessionModal({
  open,
  onClose,
  onCreate,
}: V2NewSessionModalProps): React.JSX.Element {
  const comparators = listComparators();
  const [draftKind, setDraftKind] = useState<SessionKind>("text");
  const [draftLeft, setDraftLeft] = useState("");
  const [draftRight, setDraftRight] = useState("");

  return (
    <V2Modal
      open={open}
      title="New Comparison Session"
      subtitle="Pick a mode and optional paths (leave blank for demo)"
      widthClass="max-w-md"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className="v2-btn">
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              onCreate(draftKind, draftLeft.trim(), draftRight.trim());
              setDraftLeft("");
              setDraftRight("");
              onClose();
            }}
            className="v2-btn v2-btn-primary"
          >
            Open Comparison
          </button>
        </>
      }
    >
      <div className="space-y-3 text-[12px]">
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Comparison Type
          </span>
          <select
            value={draftKind}
            onChange={(e) => setDraftKind(e.target.value as SessionKind)}
            className="v2-input w-full"
          >
            {comparators.map((plugin) => (
              <option key={plugin.id} value={plugin.id}>
                {plugin.title}
              </option>
            ))}
          </select>
        </div>
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Left Path
          </span>
          <input
            type="text"
            value={draftLeft}
            onChange={(e) => setDraftLeft(e.target.value)}
            placeholder="Optional — leave blank for sample diff"
            className="v2-input v2-mono w-full"
          />
        </div>
        <div>
          <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
            Right Path
          </span>
          <input
            type="text"
            value={draftRight}
            onChange={(e) => setDraftRight(e.target.value)}
            placeholder="Optional — leave blank for sample diff"
            className="v2-input v2-mono w-full"
          />
        </div>
      </div>
    </V2Modal>
  );
}
