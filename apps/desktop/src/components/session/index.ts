import * as React from "react";
import * as ReactDOM from "react-dom/client";
import { SessionManager, type SessionManagerProps } from "./SessionManager";

export { SessionManager, type SessionManagerProps } from "./SessionManager";

export {
  useSession,
  pickOpenDialog,
  pickSaveDialog,
  dedupSessionsById,
  sessionDisplayName,
  sessionRowKey,
  isEmptySessionIdentity,
  collapseSessionRows,
  SESSION_RESTORE_LIMIT,
  type UseSessionReturn,
} from "./useSession";

export type {
  CachedSession,
  SessionId,
  PageRequest,
  PageResponse,
} from "../../ipc/types";

export type {
  SessionDocument,
  SessionTab,
  SessionEntry,
  AppSettings,
  GrammarPreset,
  ReportFormat,
} from "../../ipc/session";

export { React, ReactDOM };

/**
 * Mount helper for testing and programmatic host environments.
 */
export function mountSessionManager(
  container: HTMLElement,
  props: SessionManagerProps = { isOpen: true },
): ReactDOM.Root {
  const host = container as HTMLElement & { __acSessionRoot?: ReactDOM.Root };
  if (!host.__acSessionRoot) {
    host.__acSessionRoot = ReactDOM.createRoot(container);
  }
  host.__acSessionRoot.render(React.createElement(SessionManager, props));
  return host.__acSessionRoot;
}
