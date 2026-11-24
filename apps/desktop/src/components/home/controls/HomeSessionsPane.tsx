/**
 * Wave-74: Home two-pane left session list (Beyond Compare Home style).
 *
 * Presentational only: rows render from the existing `visibleSessions`
 * data already derived in `App` (title + kind + dirty dot, click
 * activates via the existing `handleSelectTab` path). NO new session
 * kinds, NO backend/IPC change, NO new HOME_CARDS — the right pane keeps
 * the existing cards grid untouched.
 */
export interface HomeSessionEntry {
  readonly id: string;
  readonly title: string;
  readonly kind: string;
  readonly dirty: boolean;
}

export interface HomeSessionsPaneProps {
  readonly sessions: readonly HomeSessionEntry[];
  /** Currently active session id (null when Home itself is selected). */
  readonly activeId: string | null;
  readonly onSelect: (id: string) => void;
}

export default function HomeSessionsPane(props: HomeSessionsPaneProps): React.JSX.Element {
  const { sessions, activeId, onSelect } = props;
  return (
    <section
      aria-label="Open sessions"
      data-testid="home/sessions"
      className="w-60 shrink-0 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] p-3 text-left"
    >
      <h2 className="ac-chrome-label px-1 pb-2 font-medium text-[var(--ac-fg)]">
        Sessions{" "}
        <span className="ac-mono font-normal text-[var(--ac-fg-muted)]">{sessions.length}</span>
      </h2>
      {sessions.length === 0 ? (
        <p data-testid="home/sessions-empty" className="ac-meta px-1 py-2 text-[var(--ac-fg-muted)]">
          No open sessions yet — pick a comparison on the right to start.
        </p>
      ) : (
        <ul className="flex flex-col gap-1">
          {sessions.map((s) => {
            const selected = s.id === activeId;
            return (
              <li key={s.id}>
                <button
                  type="button"
                  data-testid={`home/session-row-${s.id}`}
                  aria-current={selected}
                  aria-label={`${s.title} (${s.kind})${s.dirty ? ", unsaved changes" : ""}`}
                  title={`${s.title} — ${s.kind}`}
                  onClick={() => onSelect(s.id)}
                  className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] ${
                    selected ? "bg-[var(--ac-toolbar-btn-hover-bg)]" : ""
                  }`}
                >
                  <span className="min-w-0 flex-1">
                    <span className="ac-chrome-label block truncate font-medium text-[var(--ac-fg)]">
                      {s.title}
                    </span>
                    <span
                      data-testid={`home/session-kind-${s.id}`}
                      className="ac-meta block truncate text-[var(--ac-fg-muted)]"
                    >
                      {s.kind}
                    </span>
                  </span>
                  {s.dirty ? (
                    <span
                      data-testid={`home/session-dirty-${s.id}`}
                      aria-label="unsaved changes"
                      title="Unsaved changes"
                      className="inline-block size-1.5 shrink-0 rounded-full bg-[var(--ac-tab-dirty)]"
                    />
                  ) : null}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
