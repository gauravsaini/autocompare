/**
 * Pure frontend mirror of `crates/sessions/src/event_log.rs` (Phase 5).
 *
 * Provides immutable append-only session event logging with linear
 * time-travel undo/redo and JSONL export for Sync & Merge views.
 */

export interface UiSessionEvent<TPayload = unknown> {
  readonly seq: number;
  readonly timestampMs: number;
  readonly action: string;
  readonly label: string;
  readonly payload: TPayload;
}

export interface UiEventLogState<TPayload = unknown> {
  readonly events: readonly UiSessionEvent<TPayload>[];
  readonly cursor: number;
}

export function createEventLog<TPayload = unknown>(): UiEventLogState<TPayload> {
  return { events: [], cursor: 0 };
}

export function appendEvent<TPayload>(
  log: UiEventLogState<TPayload>,
  action: string,
  label: string,
  payload: TPayload,
): UiEventLogState<TPayload> {
  const active = log.events.slice(0, log.cursor);
  const nextSeq = active.length > 0 ? active[active.length - 1].seq + 1 : 1;
  const nextEvent: UiSessionEvent<TPayload> = {
    seq: nextSeq,
    timestampMs: Date.now(),
    action,
    label,
    payload,
  };
  const nextEvents = [...active, nextEvent];
  return { events: nextEvents, cursor: nextEvents.length };
}

export function canUndoEvent<TPayload>(log: UiEventLogState<TPayload>): boolean {
  return log.cursor > 0;
}

export function canRedoEvent<TPayload>(log: UiEventLogState<TPayload>): boolean {
  return log.cursor < log.events.length;
}

export function undoEvent<TPayload>(log: UiEventLogState<TPayload>): {
  next: UiEventLogState<TPayload>;
  undone: UiSessionEvent<TPayload> | null;
} {
  if (log.cursor <= 0) return { next: log, undone: null };
  const nextCursor = log.cursor - 1;
  return {
    next: { events: log.events, cursor: nextCursor },
    undone: log.events[nextCursor] ?? null,
  };
}

export function redoEvent<TPayload>(log: UiEventLogState<TPayload>): {
  next: UiEventLogState<TPayload>;
  redone: UiSessionEvent<TPayload> | null;
} {
  if (log.cursor >= log.events.length) return { next: log, redone: null };
  const redone = log.events[log.cursor] ?? null;
  return {
    next: { events: log.events, cursor: log.cursor + 1 },
    redone,
  };
}

export function activeEvents<TPayload>(
  log: UiEventLogState<TPayload>,
): readonly UiSessionEvent<TPayload>[] {
  return log.events.slice(0, log.cursor);
}

export function exportEventLogJsonl<TPayload>(log: UiEventLogState<TPayload>): string {
  return activeEvents(log)
    .map((ev) => JSON.stringify(ev))
    .join("\n");
}
