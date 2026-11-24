# Wave-2C Session Management & Mount Contract

**Package**: `apps/desktop/src/components/session`  
**Target Consumer**: Wave-2R (Desktop Shell & Workspace Router)  
**Status**: Ready for Integration  

---

## 1. Overview & Exports

Wave-2C delivers the session management subsystem for the AutoCompare desktop application. It integrates typed IPC session commands (`session_list`, `session_load`, `session_save`, and graceful `session_delete`) with native file dialogs via `@tauri-apps/plugin-dialog`.

### Module Exports (`src/components/session/index.ts`)

```typescript
// Component
export { SessionManager, type SessionManagerProps } from "./SessionManager";

// Hook & Dialog Helpers
export { useSession, pickOpenDialog, pickSaveDialog, type UseSessionReturn } from "./useSession";

// Wave-5A identity + Wave-6A restore-collapse helpers
export {
  dedupSessionsById,
  sessionDisplayName,
  sessionRowKey,
  isEmptySessionIdentity,
  collapseSessionRows,
  SESSION_RESTORE_LIMIT,
} from "./useSession";

// Domain & IPC Types
export type { CachedSession, SessionId, PageRequest, PageResponse } from "../../ipc/types";
export type { SessionDocument, SessionTab, SessionEntry, AppSettings, GrammarPreset, ReportFormat } from "../../ipc/session";
```

---

## 2. `useSession` Hook Contract

```typescript
function useSession(): UseSessionReturn;
```

### Hook Return Shape

```typescript
export interface UseSessionReturn {
  /** Available cached and in-memory sessions */
  readonly sessions: CachedSession[];

  /** Currently active session identifier, or null */
  readonly activeId: SessionId | null;

  /** Active session object, or null */
  readonly activeSession: CachedSession | null;

  /**
   * Opens or creates a new session in-memory and sets it active.
   * Signature: openSession(kind, left?, right?)
   * Returns: Promise<string | null> (minted session id, or null on error)
   */
  readonly openSession: (
    kind: string,
    left?: string | null,
    right?: string | null,
  ) => Promise<string | null>;

  /**
   * Closes a session by id and activates a remaining session.
   * Signature: closeSession(id)
   * Returns: Promise<boolean>
   */
  readonly closeSession: (id: string) => Promise<boolean>;

  /**
   * Saves a session via session_save IPC. If path is omitted, invokes native save dialog.
   * Signature: saveSession(id?, path?)
   * Returns: Promise<string | null> (saved session id, or null on error)
   */
  readonly saveSession: (id?: string, path?: string) => Promise<string | null>;

  /**
   * Loads a saved session from disk via session_load IPC. If path is omitted, invokes native open dialog.
   * Signature: loadSession(path?)
   * Returns: Promise<string | null> (loaded session id, or null on error)
   */
  readonly loadSession: (path?: string) => Promise<string | null>;

  /**
   * Refreshes the session list from session_list IPC.
   * Signature: refresh()
   * Returns: Promise<void>
   */
  readonly refresh: () => Promise<void>;

  /**
   * Deletes a session by id from store and active state.
   * Signature: deleteSession(id)
   * Returns: Promise<boolean>
   */
  readonly deleteSession: (id: string) => Promise<boolean>;

  /**
   * Selects and activates a session by id.
   * Signature: selectSession(id)
   */
  readonly selectSession: (id: string | null) => void;

  /**
   * Inline error string if any operation fails. Guaranteed: never throws uncaught errors to UI.
   */
  readonly error: string | null;

  /** Clears the current inline error */
  readonly clearError: () => void;
}
```

### Error Handling Guarantees
- All IPC and dialog invocations are wrapped in defensive try/catch blocks.
- Errors are normalized and stored as inline strings in `error`.
- The hook **never throws** uncaught exceptions to UI components or event loops.

---

## 3. `SessionManager` Dialog Component Contract

The `SessionManager` component renders a modal dialog for managing recent sessions, creating new comparison sessions, and executing open/save/delete actions.

### Props Interface

```typescript
export interface SessionManagerProps {
  /** Controls dialog open/close visibility. Defaults to true. */
  readonly isOpen?: boolean;

  /** Invoked when user clicks backdrop or close button. */
  readonly onClose?: () => void;

  /**
   * Controlled active session ID.
   * If omitted, managed internally by the hook.
   */
  readonly activeId?: string | null;

  /**
   * Controlled sessions list.
   * If omitted, fetched automatically via useSession().
   */
  readonly sessions?: readonly CachedSession[];

  /**
   * Callback fired whenever the active session or selection changes.
   * Called with the active CachedSession or null.
   */
  readonly onChange?: (activeSession: CachedSession | null) => void;

  /** Custom handler for opening/creating a new session */
  readonly onOpenSession?: (
    kind: string,
    left?: string | null,
    right?: string | null,
  ) => Promise<string | null> | void;

  /** Custom handler for closing a session */
  readonly onCloseSession?: (id: string) => Promise<boolean> | void;

  /** Custom handler for saving a session */
  readonly onSaveSession?: (id?: string, path?: string) => Promise<string | null> | void;

  /** Custom handler for loading a session */
  readonly onLoadSession?: (path?: string) => Promise<string | null> | void;

  /** Custom handler for deleting a session */
  readonly onDeleteSession?: (id: string) => Promise<boolean> | void;

  /** Additional styling classes for dialog container */
  readonly className?: string;
}
```

---

## 4. Required `data-testid` Selectors

For automated testing and end-to-end verification, the following selectors are guaranteed:

| Selector | Element Description |
|---|---|
| `data-testid="session/dialog"` | Main modal dialog container |
| `data-testid="session/list"` | Scrollable list container of recent sessions |
| `data-testid="session/new"` | New session creation form container |
| `data-testid="session/open"` | Toolbar button to open/load session from file |
| `data-testid="session/save"` | Toolbar button to save the current session (disabled if no active session) |
| `data-testid="session/delete"` | Toolbar button to delete the selected session (disabled if no active session) |
| `data-testid="session/kind-select"` | Dropdown selector for session kind (`text`, `folder`, `hex`, `table`, `media`, `merge`, `sync`) |
| `data-testid="session/left-path"` | Text input for left file/folder path |
| `data-testid="session/right-path"` | Text input for right file/folder path |
| `data-testid="session/create-button"` | Submit button to create and activate the new session |

---

## 5. Wave-2R Mount Recipe

Wave-2R can mount `SessionManager` in the desktop application shell as follows:

```tsx
import React, { useState } from "react";
import { SessionManager, useSession, type CachedSession } from "./components/session";

export function AppShell() {
  const [isManagerOpen, setIsManagerOpen] = useState(false);
  const {
    sessions,
    activeId,
    activeSession,
    openSession,
    saveSession,
    loadSession,
    deleteSession,
    selectSession,
    error,
  } = useSession();

  const handleSessionChange = (session: CachedSession | null) => {
    if (session) {
      selectSession(session.id);
    }
  };

  return (
    <div className="flex h-screen flex-col">
      {/* Shell Header / Menu */}
      <header className="flex items-center justify-between px-4 py-2 border-b">
        <div>AutoCompare - {activeSession ? activeSession.name : "No Session"}</div>
        <button onClick={() => setIsManagerOpen(true)}>Manage Sessions</button>
      </header>

      {/* Main Comparison Workspace */}
      <main className="flex-1">
        {activeSession ? (
          <div>Render View for {activeSession.kind} ({activeSession.id})</div>
        ) : (
          <div>Please open or create a session.</div>
        )}
      </main>

      {/* Session Manager Modal */}
      <SessionManager
        isOpen={isManagerOpen}
        onClose={() => setIsManagerOpen(false)}
        activeId={activeId}
        sessions={sessions}
        onChange={handleSessionChange}
      />
    </div>
  );
}
```

---

## 6. Wave-5A Dedup Addendum (tab + session identity)

Root cause fixed: `openSession` used to mint a fresh `sess-mem-*` id on every
call, so re-opening identical kind+paths stacked duplicate sessions/tabs
instead of focusing the existing one.

Rules now enforced (see `useSession.ts`):

1. `openSession(kind, left, right)` focus-existing: when a session with the
   same normalized `(kind, left, right)` triple already exists (matched via
   the descriptor cache, falling back to kind+display-name), it is activated
   and its id returned — no new session, no new tab.
2. Every list merge (`refresh`, open, load, dialog sync) dedups by session
   id, first-wins, order-preserving (`dedupSessionsById`).
3. `closeSession`/`deleteSession` remove exactly one id and compute the next
   active tab from the post-removal list (never a stale render scope).
4. Last-active restore: the active id persists to
   `localStorage["autocompare:lastActiveSessionId"]` (best-effort, never
   throws); `refresh` re-selects it when still present, else the first row.
5. `App.tsx` renders tabs from the id-deduped session list, so one session
   always renders exactly one tab; the welcome CTA opens SessionManager even
   at zero sessions.

## 7. Wave-6A Legacy-Pile Cleanup (restore/load merge + cap)

Root cause of the 24-row pile: `session_list` returns one row per `*.json`
file with no kind/paths, and `refresh` used to map every row to
`kind: "text"` verbatim — so N same-name files rendered N identical
sessions/tabs even though most shared one identity. Wave-5A stopped NEW
dupes (open focus-existing); Wave-6A collapses the LEGACY pile on restore.

Rules now enforced (see `useSession.ts`):

1. `refresh()` resolves every persisted row to its real kind+paths with a
   real `session_load` per entry (best-effort; unloadable entries keep a
   unique per-id key so unknown identity is preserved, never merged and
   never dropped), then runs the pure `collapseSessionRows` over the
   combined persisted + in-memory list:
   - identical normalized kind+paths (`sessionRowKey`: lowercased kind,
     slash-normalized paths, newline-separated so `text|/a|/b` can never
     collide with `text/a||/b`) merge into ONE row, keeping the newest id
     (`sess-<millis>-<counter>` sorts lexicographically newest-first);
   - rows with empty kind+paths (vacuous doc: blank kind AND blank paths,
     or zero tabs) are dropped from the list (on-disk files untouched);
   - the restore window caps at the newest `SESSION_RESTORE_LIMIT` (20)
     rows; overflow stays on disk, unlisted.
2. Hidden rows' descriptor-cache entries are pruned so focus-existing can
   never match an unlisted tab; kept persisted docs are cached so a later
   `openSession` with the same kind+paths focuses instead of duplicating.
3. `saveSession` retires its source row (`retiredIdsRef`): the backend
   mints a fresh id per save, so without retirement the source row and
   its persisted twin would share one identity and the collapse would hide
   the id `saveSession` just activated.
4. `loadSession` drops only rows with PROVEN equal identity (doc-cached
   kind+paths match); unknown rows are always preserved.
5. `closeSession`/`deleteSession` still remove exactly one row by id and
   compute the next active tab from the post-removal list.
6. Every collapse reports `merged + droppedEmpty` via
   `useSession().cleanupCount`. `SessionManager` renders a transient
   `session/cleanup-note` ("Cleaned up N duplicate sessions", re-armed on
   count change and dialog open, auto-dismissed after 6s, no new deps);
   `App.tsx` passes its hook count through as `cleanupCount` (session
   wiring only).

## 8. Gaps & Integration Notes for Wave-2R

1. **`session_delete` Backend Registration** (unchanged):
   - `Wave 1R` registered `session_list`, `session_load`, `session_save`, `settings_get`, `settings_set`, and `report_export` in `src-tauri/src/lib.rs`.
   - `session_delete` was not implemented in `src-tauri/src/commands/session.rs` or `lib.rs`.
   - In Wave-2C, `useSession.deleteSession()` attempts `invoke("session_delete", { id })` inside a defensive handler, logs a warning on failure, and updates frontend memory state cleanly without throwing. Wave-2R should wire up `session_delete` in Rust when persistent file deletion is needed.

2. **Native Dialog vs Mock IPC**:
   - Under `VITE_MOCK_IPC=1` or in test environments without Tauri internals, `pickOpenDialog` and `pickSaveDialog` safely fallback to window mock overrides (`window.__MOCK_DIALOG_OPEN__`, `window.__MOCK_DIALOG_SAVE__`) or resolve `null` without throwing.
