import React, { useState } from "react";
import { Copy, History, Redo2, Undo2 } from "lucide-react";
import {
  activeEvents,
  canRedoEvent,
  canUndoEvent,
  exportEventLogJsonl,
  type UiEventLogState,
} from "../utils/sessionEventLog";

export interface V2EventLogBarProps<TPayload> {
  readonly log: UiEventLogState<TPayload>;
  readonly onUndo: () => void;
  readonly onRedo: () => void;
  readonly onCopied?: (msg: string) => void;
}

export default function V2EventLogBar<TPayload>({
  log,
  onUndo,
  onRedo,
  onCopied,
}: V2EventLogBarProps<TPayload>): React.JSX.Element {
  const [expanded, setExpanded] = useState(false);
  const active = activeEvents(log);
  const canUndo = canUndoEvent(log);
  const canRedo = canRedoEvent(log);

  const handleCopyJsonl = (): void => {
    const jsonl = exportEventLogJsonl(log);
    void navigator.clipboard?.writeText(jsonl).then(
      () => onCopied?.(`Copied ${active.length} event(s) as JSONL`),
      () => onCopied?.("Clipboard unavailable"),
    );
  };

  return (
    <div
      data-testid="v2/event-log-bar"
      className="shrink-0 border-t border-[var(--v2-border)] bg-[var(--v2-chrome)] text-[11px]"
    >
      <div className="flex items-center justify-between gap-2 px-3 py-1">
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            disabled={!canUndo}
            onClick={onUndo}
            title="Undo last action"
            className="v2-btn h-[21px] px-2 text-[11px]"
          >
            <Undo2 size={11} />
            Undo
          </button>
          <button
            type="button"
            disabled={!canRedo}
            onClick={onRedo}
            title="Redo action"
            className="v2-btn h-[21px] px-2 text-[11px]"
          >
            <Redo2 size={11} />
            Redo
          </button>
          <button
            type="button"
            onClick={() => setExpanded((e) => !e)}
            className="v2-btn h-[21px] px-2 text-[11px]"
          >
            <History size={11} />
            Audit Log ({active.length})
          </button>
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-[var(--v2-fg-muted)]">
            {active.length > 0 ? active[active.length - 1].label : "No recorded actions yet"}
          </span>
          {active.length > 0 ? (
            <button
              type="button"
              onClick={handleCopyJsonl}
              title="Copy session event log as JSONL"
              className="v2-btn h-[21px] shrink-0 px-2 text-[10.5px]"
            >
              <Copy size={10} />
              JSONL
            </button>
          ) : null}
        </div>
      </div>

      {expanded ? (
        <div className="v2-mono max-h-[110px] overflow-y-auto border-t border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-3 py-1.5 text-[10.5px] leading-[16px]">
          {log.events.length === 0 ? (
            <div className="text-[var(--v2-fg-muted)]">Event log is empty.</div>
          ) : (
            log.events.map((ev, idx) => {
              const isUndone = idx >= log.cursor;
              return (
                <div
                  key={`${ev.seq}-${idx}`}
                  className={`flex items-center justify-between gap-2 ${
                    isUndone ? "opacity-40 line-through" : "text-[var(--v2-fg)]"
                  }`}
                >
                  <span>
                    #{ev.seq} [{ev.action}] {ev.label}
                  </span>
                  <span className="text-[var(--v2-fg-muted)]">
                    {new Date(ev.timestampMs).toLocaleTimeString()}
                  </span>
                </div>
              );
            })
          )}
        </div>
      ) : null}
    </div>
  );
}
