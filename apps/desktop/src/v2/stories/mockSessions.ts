import type { SessionDocument } from "../../ipc/session";

const MOCK_SESSIONS_KEY = "autocompare:mockSessionStore:v1";
const PINNED_SESSIONS_KEY = "autocompare:v2:pinnedSessions";
const SIDEBAR_PREFS_KEY = "autocompare.v2.home.sidebar.v1";

export const SAMPLE_SAVED_SESSIONS: Record<string, SessionDocument> = {
  "sess-api-diff": {
    schema_version: 1,
    name: "API Config JSON Diff",
    locked: true,
    active_tab_index: 0,
    tabs: [
      {
        id: "t1",
        title: "staging.json ↔ prod.json",
        session_type: "structured",
        left_path: "/configs/staging.json",
        right_path: "/configs/prod.json",
        base_path: null,
        output_path: null,
        read_only: false,
        locked: true,
        options: {},
      },
    ],
  },
  "sess-release-folder": {
    schema_version: 1,
    name: "Release Build Folder Check",
    locked: false,
    active_tab_index: 0,
    tabs: [
      {
        id: "t2",
        title: "dist-v0.5 ↔ dist-v0.6",
        session_type: "folder",
        left_path: "tools/parity-harness/fixtures/folder/left",
        right_path: "tools/parity-harness/fixtures/folder/right",
        base_path: null,
        output_path: null,
        read_only: false,
        locked: false,
        options: {},
      },
    ],
  },
  "sess-notes-text": {
    schema_version: 1,
    name: "Core Diff Text Baseline",
    locked: false,
    active_tab_index: 0,
    tabs: [
      {
        id: "t3",
        title: "t_left.txt ↔ t_right.txt",
        session_type: "text",
        left_path: "tools/parity-harness/fixtures/t_left.txt",
        right_path: "tools/parity-harness/fixtures/t_right.txt",
        base_path: null,
        output_path: null,
        read_only: false,
        locked: false,
        options: {},
      },
    ],
  },
};

export function seedStorySessions(opts?: {
  readonly empty?: boolean;
  readonly pinnedIds?: readonly string[];
  readonly sidebarOpen?: boolean;
  readonly sidebarWidth?: number;
}): void {
  if (typeof window === "undefined" || !window.localStorage) return;
  if (opts?.empty) {
    window.localStorage.setItem(MOCK_SESSIONS_KEY, JSON.stringify({}));
    window.localStorage.setItem(PINNED_SESSIONS_KEY, JSON.stringify([]));
  } else {
    window.localStorage.setItem(MOCK_SESSIONS_KEY, JSON.stringify(SAMPLE_SAVED_SESSIONS));
    window.localStorage.setItem(
      PINNED_SESSIONS_KEY,
      JSON.stringify(opts?.pinnedIds ?? ["sess-api-diff"]),
    );
  }
  if (opts?.sidebarOpen !== undefined || opts?.sidebarWidth !== undefined) {
    window.localStorage.setItem(
      SIDEBAR_PREFS_KEY,
      JSON.stringify({
        open: opts.sidebarOpen ?? true,
        width: opts.sidebarWidth ?? 268,
      }),
    );
  }
}
