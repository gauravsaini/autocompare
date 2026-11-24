import React, { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeftRight, Binary, ChevronDown, ChevronUp, Columns2, Copy, FileText, Folder, FolderOpen, GitMerge, Image, RefreshCw, Table } from "lucide-react";
import { ChromeShell, rowIndexToScrollTop, textCompareKindToDiffKind } from "./components/chrome";
import { DiffStatsContext, type DiffStats } from "./components/chrome/StatusBar";
import type { ChromeTab, DiffRow, TextCompareRowKind, ToolbarAction } from "./components/chrome";
import { LineDetailPane } from "./components/chrome/LineDetailPane";
import { ViewRouter, type DataViewKind } from "./components/views";
import { SessionManager, useSession, dedupSessionsById, type CachedSession } from "./components/session";
import { textCompare, textCompareDemo, textGetOverview, textGetRows } from "./ipc/text";
import type { TextRowDto, TextSide } from "./ipc/text";
import { sessionList, sessionLoad } from "./ipc/session";
import { folderCompare } from "./ipc/folder";
import { hexCompare } from "./ipc/hex";
import { tableCompare } from "./ipc/table";
import { pictureCompare } from "./ipc/media";
import { HomeSessionsPane } from "./components/home/controls";
import { PaneDropZone, useTauriPaneDrop, isFolderishPath, type PaneSide } from "./components/dnd";
import { syncPlan } from "./ipc/sync";

/**
 * Wave 2R desktop workspace (APP INTEGRATION REGISTRY).
 *
 * Wiring map (Wave 2A chrome + Wave 2B views + Wave 2C sessions):
 * - Tabs derive from `useSession` sessions; a `home` welcome tab shows
 *   when no sessions are open. Selecting/closing tabs delegates to
 *   `selectSession`/`closeSession`.
 * - The ChromeShell view slot (`children`) mounts `ViewRouter` for the
 *   active tab (`kind`/`leftPath`/`rightPath`/`sessionId` from tab state).
 * - Toolbar: Open launches the SessionManager dialog; Compare re-runs the
 *   kind-matched IPC compare; Copy (text row in a hunk selected) rides
 *   the TextView gutter copy path via event; Merge (merge kind only)
 *   jumps to the Merge view; Sync calls `syncPlan` in folder/sync
 *   kinds; anything else renders disabled with the reason in the label
 *   (which doubles as the `title` tooltip).
 * - SessionManager dialog is mounted open/close-controlled; `onChange`
 *   selects the session (tabs follow). All errors render inline, never
 *   thrown. Under `VITE_MOCK_IPC=1` the full workspace boots with zero
 *   sessions and every action degrades to fixtures/empty states.
 */

interface TabPaths {
  readonly kind: string;
  readonly left: string;
  readonly right: string;
  readonly base?: string;
  readonly output?: string;
  readonly options?: Record<string, string>;
}

const WELCOME_TAB: ChromeTab = { id: "home", title: "Welcome", kind: "home", dirty: false };

/**
 * Wave-53: home quick-create cards (Beyond Compare style session picker).
 * Each card opens a blank session tab through the existing
 * `handleOpenSession` path (same as the Session Manager) — no new IPC,
 * no new state shape. Label + hint render; full kind stays in title.
 */
const HOME_CARDS = [
  { kind: "text", icon: FileText, label: "Text Compare", hint: "Files, line by line" },
  { kind: "folder", icon: Folder, label: "Folder Compare", hint: "Directories and archives" },
  { kind: "hex", icon: Binary, label: "Hex Compare", hint: "Bytes and offsets" },
  { kind: "table", icon: Table, label: "Table Compare", hint: "Rows and columns" },
  { kind: "merge", icon: GitMerge, label: "Merge", hint: "Three-way merge" },
  { kind: "sync", icon: RefreshCw, label: "Sync", hint: "Mirror folders" },
  { kind: "media", icon: Image, label: "Media Compare", hint: "Picture and audio" },
] as const;

function toViewKind(kind: string): DataViewKind | null {
  switch (kind) {
    case "text":
    case "folder":
    case "hex":
    case "table":
    case "merge":
    case "sync":
    case "media":
      return kind;
    default:
      return null;
  }
}

/** Kinds whose toolbar-level Compare maps 1:1 to an `*_compare` IPC. */
function canCompareKind(kind: string): boolean {
  return kind === "text" || kind === "folder" || kind === "hex" || kind === "table" || kind === "media" || kind === "sync";
}

/**
 * Wave-120: drop kind-class (BC `TFileView` vs `TDirBaseView` parity).
 * File-kind sessions compare files (`text`/`hex`/`table`/`media`);
 * folder-kind sessions compare directories (`folder`/`sync`); anything
 * else (e.g. `merge`) is `other` — drops stage honestly, never switch.
 */
type DropKindClass = "file" | "folder" | "other";

function dropKindClass(kind: string): DropKindClass {
  if (kind === "text" || kind === "hex" || kind === "table" || kind === "media") return "file";
  if (kind === "folder" || kind === "sync") return "folder";
  return "other";
}
export default function App(): React.JSX.Element {
  const {
    sessions,
    activeId,
    activeSession,
    openSession,
    closeSession,
    saveSession,
    loadSession,
    deleteSession,
    selectSession,
    cleanupCount,
    error: sessionError,
    clearError,
  } = useSession();

  const [managerOpen, setManagerOpen] = useState(false);
  const [tabPaths, setTabPaths] = useState<Readonly<Record<string, TabPaths>>>({});
  // Wave-138: live mirror of tabPaths for back-to-back submits in one
  // tick (dual-drop fires left+right submits synchronously; the second
  // submit must see the first submit's staged side, not the stale
  // render closure, otherwise it wipes it and blank stays blank).
  const tabPathsRef = useRef<Readonly<Record<string, TabPaths>>>({});
  useEffect(() => {
    tabPathsRef.current = tabPaths;
  }, [tabPaths]);
  // Wave-33: per-tab last-run snapshot of compare inputs. A session tab is
  // dirty when its current inputs were set/edited after the last successful
  // toolbar Compare for that tab (or never ran). Derived from existing App
  // state only: no new IPC, no layout changes.
  const [comparedPaths, setComparedPaths] = useState<Readonly<Record<string, TabPaths>>>({});
  const [statusMessage, setStatusMessage] = useState<string>("Ready");
  const [actionError, setActionError] = useState<string | null>(null);
  const [compareTick, setCompareTick] = useState(0);
  // Wave-13: live ruler rows for the text pane (same equal/del/ins/chg
  // mapping TextView paints, so markers line up 1:1 with the pane).
  // Non-text kinds resolve [] (ruler empty, visual-only as before).
  const [rulerRows, setRulerRows] = useState<readonly DiffRow[]>([]);
  const [rulerActive, setRulerActive] = useState(0);
  // Wave-10 demo first-paint: auto-opened demo session id (auto-persisted
  // like any tab since wave-65; still recognised as the demo for the banner).
  // Dismissed once the user clears/closes it, so this boot never
  // re-opens the demo in a loop.
  const [demoId, setDemoId] = useState<string | null>(null);
  const [demoDismissed, setDemoDismissed] = useState(false);
  const demoTriedRef = useRef(false);
  // Wave-138: live guards for the boot demo async below (stale-closure
  // race: user tab khol sakta hai jab awaits in-flight hon).
  const liveSessionCountRef = useRef(0);
  const demoDismissedRef = useRef(false);
  useEffect(() => {
    liveSessionCountRef.current = sessions.length;
    demoDismissedRef.current = demoDismissed;
  });
  // Wave-21: bottom line-detail selection. Holds the already-loaded
  // selected text row (reported up by `TextView` on click / arrow keys,
  // no new IPC); `ChromeShell` renders it via `LineDetailPane` in the
  // bottom zone. Cleared whenever the active session changes.
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [selectedRow, setSelectedRow] = useState<TextRowDto | null>(null);

  const handleSelectRow = useCallback((index: number | null, row: TextRowDto | null) => {
    setSelectedIndex(index);
    setSelectedRow(row);
  }, []);

  // Tabs follow sessions; welcome tab only when empty.
  // Tabs follow sessions (deduped by id: one session renders exactly one
  // tab, so a duplicated store row can never multiply the tab row); a
  // `home` welcome tab shows when no sessions are open.
  const visibleSessions = dedupSessionsById(sessions);
  // Wave-33: honest dirty derivation from existing tab state only (no new
  // IPC). A session tab is dirty when it carries runnable compare inputs
  // (non-blank paths of a comparable kind) that differ from the last
  // successful toolbar Compare snapshot for that tab — fresh inputs that
  // never ran count as edited-since-last-run. Blank tabs have nothing to
  // run; non-comparable kinds (e.g. merge) have no Compare to be stale
  // against; welcome is never dirty (TabBar enforces this too). Pure
  // per-tab derivation, so switching tabs preserves each tab's dot.
  const isTabDirty = (id: string): boolean => {
    if (id === WELCOME_TAB.id) return false;
    const current = tabPaths[id];
    if (!current) return false;
    if (current.left === "" && current.right === "") return false;
    if (!canCompareKind(current.kind)) return false;
    const snap = comparedPaths[id];
    if (!snap) return true;
    return snap.kind !== current.kind || snap.left !== current.left || snap.right !== current.right;
  };
  const tabs: readonly ChromeTab[] =
    // Wave-74: the Home tab is always present (Beyond Compare keeps Home
    // reachable with sessions open). Selecting it shows the two-pane
    // welcome below with the session list populated; the welcome tab is
    // never dirty and never closable (enforced by `isTabDirty`/TabBar).
    [WELCOME_TAB, ...visibleSessions.map((s) => ({ id: s.id, title: s.name, kind: s.kind, dirty: isTabDirty(s.id) }))];

  const activeTabId =
    // Wave-74: selecting the always-present Home tab shows the welcome
    // two-pane (`selectSession("home")` leaves `activeSession` null, so
    // the welcome branch below renders with the session list populated).
    activeId === WELCOME_TAB.id
      ? WELCOME_TAB.id
      : activeId !== null && visibleSessions.some((s) => s.id === activeId)
        ? activeId
        : (visibleSessions[0]?.id ?? WELCOME_TAB.id);

  // Wave-21: switching sessions clears the line-detail selection.
  useEffect(() => {
    setSelectedIndex(null);
    setSelectedRow(null);
  }, [activeTabId]);

  const activePaths: TabPaths = tabPaths[activeTabId] ?? {
    kind: activeSession?.kind ?? WELCOME_TAB.kind,
    left: "",
    right: "",
  };

  const viewKind: DataViewKind | null = activeSession ? toViewKind(activeSession.kind) : null;

  const handleSelectTab = useCallback(
    (id: string) => {
      if (id === WELCOME_TAB.id && visibleSessions.length === 0) {
        setStatusMessage("Welcome");
        return;
      }
      selectSession(id);
      const target = visibleSessions.find((s) => s.id === id);
      setStatusMessage(target ? `Tab selected: ${target.name}` : `Tab selected: ${id}`);
    },
    [selectSession, visibleSessions],
  );

  const handleCloseTab = useCallback(
    (id: string) => {
      if (id === WELCOME_TAB.id) return;
      setTabPaths((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      // Wave-33: drop the tab's last-run snapshot with its paths so a
      // later tab never inherits a stale clean mark.
      setComparedPaths((prev) => {
        if (!(id in prev)) return prev;
        const next = { ...prev };
        delete next[id];
        return next;
      });
      void closeSession(id).then((ok) => {
        setStatusMessage(ok ? `Tab closed: ${id}` : `Close failed: ${id}`);
      });
    },
    [closeSession],
  );

  const handleNewTab = useCallback(() => {
    setManagerOpen(true);
    setStatusMessage("Session Manager opened");
  }, []);

  const handleManagerChange = useCallback(
    (session: CachedSession | null) => {
      if (session) {
        selectSession(session.id);
        // Wave-123: hydrate from the stored doc when App has no live paths
        // yet (boot/restore/select), never blank over live edits.
        setTabPaths((prev) => {
          if (prev[session.id] !== undefined) return prev;
          return { ...prev, [session.id]: { kind: session.kind, left: "", right: "" } };
        });
        void sessionLoad(session.id)
          .then((doc) => {
            const tabs = Array.isArray(doc?.tabs) ? doc.tabs : [];
            if (tabs.length === 0) return;
            const idx =
              typeof doc.active_tab_index === "number" &&
              doc.active_tab_index >= 0 &&
              doc.active_tab_index < tabs.length
                ? doc.active_tab_index
                : 0;
            const t = tabs[idx] ?? tabs[0]!;
            setTabPaths((prev) => {
              const cur = prev[session.id];
              // Keep live edits; only fill when still blank/missing.
              if (cur !== undefined && (cur.left !== "" || cur.right !== "")) return prev;
              return {
                ...prev,
                [session.id]: {
                  kind: t.session_type || session.kind,
                  left: t.left_path ?? "",
                  right: t.right_path ?? "",
                  base: t.base_path ?? undefined,
                  output: t.output_path ?? undefined,
                  options: t.options ? { ...t.options } : undefined,
                },
              };
            });
          })
          .catch(() => {
            // Best-effort hydration only; blank entry above keeps UI working.
          });
        setStatusMessage(`Session active: ${session.name}`);
      }
    },
    [selectSession],
  );

  const handleOpenSession = useCallback(
    async (kind: string, left?: string | null, right?: string | null, base?: string | null): Promise<string | null> => {
      const id = await openSession(kind, left, right, base);
      if (id) {
        setTabPaths((prev) => ({
          ...prev,
          // Wave-134: tabPaths carry the base ancestor (merge 3-way);
          // falsy base stays absent so text/folder tabs keep legacy shape.
          [id]: { kind, left: left ?? "", right: right ?? "", base: base || undefined },
        }));
        setActionError(null);
        setStatusMessage(`Session opened: ${kind}`);
      }
      return id;
    },
    [openSession],
  );

  // Wave-131: native Take Left / Take Right (menu.rs Session items,
  // accelerators CmdOrCtrl+D / CmdOrCtrl+E). Routed via `menu-event`
  // to the SAME `autocompare:text-copy` gutter path the toolbar Copy
  // uses (TextView owns `copyHunkAt`); honest status note when no
  // hunk row is selected (native items stay enabled: no new IPC to
  // toggle them, so the frontend reports instead of pretending).
  const handleMenuTake = useCallback(
    (dir: TextSide) => {
      if (selectedIndex === null || selectedRow === null || selectedRow.kind === "equal") {
        setStatusMessage(`Take ${dir} needs a hunk row selected`);
        return;
      }
      window.dispatchEvent(
        new CustomEvent("autocompare:text-copy", { detail: { index: selectedIndex, dir } }),
      );
      setStatusMessage(`Copy take-${dir} requested for row ${selectedIndex}`);
    },
    [selectedIndex, selectedRow],
  );

  // Wave-128: native menu wiring (BC parity). The Rust shell (`menu.rs`)
  // emits `menu-event` for all 18 menu items (including merge kind and
  // Wave-131 Take Left/Right), routed with honest command intents.
  // with honest command intents. Each id below maps to an existing App action;
  // unsupported actions (theme/layout/settings) report honestly instead of
  // pretending. Test mock dispatch supported via window "menu-event" and
  // `__MOCK_EMIT_MENU__`. No new IPC, no new state shape.
  useEffect(() => {
    const handleMenuPayload = (payload: { id: string; command?: string }) => {
      const id = payload?.id ?? "";
      switch (id) {
        case "file.open-session":
        case "file.open-workspace":
          setManagerOpen(true);
          setStatusMessage("Session Manager opened");
          break;
        case "file.open-folder":
          void handleOpenSession("folder");
          break;
        case "file.save-session":
          void saveSession(activeId ?? undefined, undefined, activePaths ? {
            kind: activePaths.kind,
            left: activePaths.left ?? null,
            right: activePaths.right ?? null,
            base: activePaths.base ?? null,
            output: activePaths.output ?? null,
            options: activePaths.options ? { ...activePaths.options } : null,
          } : null).then((saved) => {
            setStatusMessage(saved ? `Session saved: ${saved}` : "Nothing to save");
          });
          break;
        case "session.new":
          handleNewTab();
          break;
        case "session.find": {
          // Wave-128 / Wave-127: open-first full-session find with fallback.
          window.dispatchEvent(new CustomEvent("autocompare:session-find"));
          const checkFocus = () => {
            const target = document.querySelector<HTMLElement>(
              '[data-testid="textview/find-input"], [data-testid="hex-find-input"], [data-testid="table-find-input"], [data-testid="merge/find-input"], [data-testid="folder-name-filter"], [data-testid="syncview/name-filter"]',
            );
            if (target) {
              target.focus();
              setStatusMessage("Find focused");
              return true;
            }
            return false;
          };
          if (!checkFocus()) {
            let attempts = 0;
            const timer = window.setInterval(() => {
              attempts += 1;
              if (checkFocus()) {
                window.clearInterval(timer);
              } else if (attempts >= 15) {
                window.clearInterval(timer);
                setStatusMessage("Find is not available for this view");
              }
            }, 50);
          }
          break;
        }
        case "view.kind-text":
        case "view.kind-folder":
        case "view.kind-hex":
        case "view.kind-table":
        case "view.kind-media":
        case "view.kind-sync": {
          const kind =
            id === "view.kind-media" ? "media" : id === "view.kind-sync" ? "sync" : id.replace("view.kind-", "");
          void handleOpenSession(kind, activePaths.left || null, activePaths.right || null);
          break;
        }
        case "view.kind-merge":
          void handleOpenSession("merge", activePaths.left || null, activePaths.right || null, activePaths.base || null);
          break;
        case "session.take-left":
          handleMenuTake("left");
          break;
        case "session.take-right":
          handleMenuTake("right");
          break;
        case "view.toggle-theme":
          setStatusMessage("Theme follows the OS appearance (no in-app toggle yet)");
          break;
        case "view.toggle-layout":
          setStatusMessage("Layout toggle is not available in this build");
          break;
        case "tools.settings":
          setStatusMessage("Settings UI is not available in this build");
          break;
        default:
          break;
      }
    };

    const handleCustomMenuEvent = (event: Event) => {
      const customEvent = event as CustomEvent<{ id: string; command?: string }>;
      if (customEvent.detail) {
        handleMenuPayload(customEvent.detail);
      }
    };
    window.addEventListener("menu-event", handleCustomMenuEvent);
    (window as unknown as { __MOCK_EMIT_MENU__?: (payload: { id: string; command?: string }) => void }).__MOCK_EMIT_MENU__ = handleMenuPayload;

    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        unlisten = await listen<{ id: string; command: string }>("menu-event", (event) => {
          if (event.payload) {
            handleMenuPayload(event.payload);
          }
        });
      } catch {
        // Plain-browser / mock runs have no Tauri event bridge; menus
        // simply stay inert there, same as before.
      }
    })();
    return () => {
      window.removeEventListener("menu-event", handleCustomMenuEvent);
      delete (window as unknown as { __MOCK_EMIT_MENU__?: unknown }).__MOCK_EMIT_MENU__;
      if (unlisten) unlisten();
    };
    // Active-tab context only; handler identity stays stable via refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeId, activePaths.left, activePaths.right, handleMenuTake]);

  // Wave-46: folder drilldown. `FolderView` double-click opens the file
  // pair in a new text session (tab) through the same `openSession`
  // path the Session Manager uses — no new IPC, no new state shape.
  const handleOpenPaths = useCallback(
    (kind: DataViewKind, left: string, right: string): void => {
      void handleOpenSession(kind, left, right);
    },
    [handleOpenSession],
  );

  const handleLoadSession = useCallback(
    async (path?: string): Promise<string | null> => {
      // Wave-123: hydrate ALL fields from the real doc (kind + left/right/
      // base/output_path + options + active_tab_index). No text-hardcode,
      // no blank fallback: a failed load returns null with inline error.
      const id = await loadSession(path);
      if (!id) return null;
      try {
        const doc = await sessionLoad(id);
        const tabs = Array.isArray(doc?.tabs) ? doc.tabs : [];
        const idx =
          typeof doc?.active_tab_index === "number" &&
          doc.active_tab_index >= 0 &&
          doc.active_tab_index < tabs.length
            ? doc.active_tab_index
            : 0;
        const t = tabs[idx] ?? tabs[0];
        if (t) {
          setTabPaths((prev) => ({
            ...prev,
            [id]: {
              kind: t.session_type || "text",
              left: t.left_path ?? "",
              right: t.right_path ?? "",
              base: t.base_path ?? undefined,
              output: t.output_path ?? undefined,
              options: t.options ? { ...t.options } : undefined,
            },
          }));
        }
      } catch {
        // loadSession already surfaced the inline error; keep existing
        // tabPaths untouched rather than blanking the pane.
      }
      setStatusMessage(`Session loaded: ${id}`);
      return id;
    },
    [loadSession],
  );

  // Wave-123: save persists live tabPaths + options. The dialog path is
  // honestly ignored by session_save (mints id); noted in useSession.
  const handleSaveSession = useCallback(
    async (id?: string, path?: string): Promise<string | null> => {
      const targetId = id ?? activeId ?? undefined;
      const live = targetId ? tabPaths[targetId] : undefined;
      const saved = await saveSession(targetId, path, live ? {
        kind: live.kind,
        left: live.left ?? null,
        right: live.right ?? null,
        base: live.base ?? null,
        output: live.output ?? null,
        options: live.options ? { ...live.options } : null,
      } : null);
      if (saved && targetId) {
        // Keep the live paths under the minted id so a reload restores them.
        const src = tabPaths[targetId];
        if (src && saved !== targetId) {
          setTabPaths((prev) => ({ ...prev, [saved]: src }));
        }
      }
      return saved;
    },
    [activeId, saveSession, tabPaths],
  );

  // Wave-123: boot/restore hydration. tabPaths is memory-only, so after a
  // reload every restored session would render blank. Fill missing entries
  // from the persisted doc's active tab (best-effort, never overwrites
  // live edits).
  useEffect(() => {
    const missing = visibleSessions.filter((s) => tabPaths[s.id] === undefined);
    if (missing.length === 0) return;
    let cancelled = false;
    void (async () => {
      for (const s of missing) {
        try {
          const doc = await sessionLoad(s.id);
          if (cancelled) return;
          const tabs = Array.isArray(doc?.tabs) ? doc.tabs : [];
          if (tabs.length === 0) continue;
          const idx =
            typeof doc.active_tab_index === "number" &&
            doc.active_tab_index >= 0 &&
            doc.active_tab_index < tabs.length
              ? doc.active_tab_index
              : 0;
          const t = tabs[idx] ?? tabs[0]!;
          setTabPaths((prev) => {
            if (prev[s.id] !== undefined) return prev;
            return {
              ...prev,
              [s.id]: {
                kind: t.session_type || s.kind,
                left: t.left_path ?? "",
                right: t.right_path ?? "",
                base: t.base_path ?? undefined,
                output: t.output_path ?? undefined,
                options: t.options ? { ...t.options } : undefined,
              },
            };
          });
        } catch {
          // Unloadable row keeps its blank entry; list still renders.
        }
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessions.length]);

  // Wave-10 demo first-paint: no persisted sessions -> auto-open the
  // bundled demo text pair via the real text_compare IPC (same
  // candidate-pair path TextView uses: textCompareDemo). Runs once;
  // openSession focus-existing keeps reloads dupe-free.
  useEffect(() => {
    if (demoTriedRef.current || demoDismissed) return;
    if (sessions.length > 0) return;
    demoTriedRef.current = true;
    void (async () => {
      try {
        const persisted = await sessionList().catch(() => null);
        if (persisted !== null && persisted.length > 0) return;
      } catch {
        return;
      }
      // Wave-138: demo SIRF tab-less/welcome boot par — awaits ke
      // dauran khula blank session demo ko rok deta hai.
      if (demoDismissedRef.current || liveSessionCountRef.current > 0) return;
      let found: { readonly left: string; readonly right: string } | null = null;
      try {
        found = await textCompareDemo();
      } catch {
        found = null;
      }
      if (!found) return;
      if (demoDismissedRef.current || liveSessionCountRef.current > 0) return;
      // TMP-138DEBUG
      console.log(`[138demo] opening demo live=${liveSessionCountRef.current} dismissed=${demoDismissedRef.current}`);
      const pair = found;
      const id = await openSession("text", pair.left, pair.right);
      if (id) {
        setDemoId(id);
        setTabPaths((prev) => ({
          ...prev,
          [id]: { kind: "text", left: pair.left, right: pair.right },
        }));
        setStatusMessage("Demo session ready");
      }
    })();
  }, [sessions.length, demoDismissed, openSession]);

  // Wave-10: closing/clearing the demo leaves a clean blank session and
  // this boot never re-opens it (reload boots a fresh demo at most once
  // via focus-existing; 6A dedup untouched).
  useEffect(() => {
    if (demoId !== null && !sessions.some((s) => s.id === demoId)) {
      setDemoId(null);
      setDemoDismissed(true);
    }
  }, [demoId, sessions]);

  // Wave-13: build DiffRow[] for the live text diff. The TextView pane
  // owns its paging session internally (views/** is untouched this
  // wave), so the ruler compares the SAME inputs through text_compare
  // and expands the full map via text_get_overview, falling back to
  // paging text_get_rows when the overview is unavailable (e.g. mock
  // IPC, which serves rows but no overview). Same inputs + same
  // mapping => markers line up 1:1 with the virtualized pane rows.
  useEffect(() => {
    if (viewKind !== "text") {
      setRulerRows([]);
      setRulerActive(0);
      return;
    }
    let cancelled = false;
    const left = activePaths.left;
    const right = activePaths.right;
    void (async () => {
      try {
        // Wave-138: blank paths never fire demo/compare here — the
        // blank session renders the honest pick-two empty (below) and
        // the ruler stays honestly at 0 rows. Demo fires SIRF the
        // tab-less/welcome boot effect above.
        if (left === "" || right === "") {
          if (!cancelled) {
            setRulerRows([]);
            setRulerActive(0);
          }
          return;
        }
        const summary = await textCompare(left, right);
        if (cancelled || summary === null) {
          if (!cancelled) setRulerRows([]);
          return;
        }
        const total = summary.rows;
        // Prefer the coarse whole-comparison map (one IPC round trip).
        try {
          const over = (await textGetOverview(summary.session)) as unknown;
          if (
            typeof over === "object" &&
            over !== null &&
            typeof (over as { total?: unknown }).total === "number" &&
            Array.isArray((over as { chunks?: unknown }).chunks)
          ) {
            const chunks = (over as { chunks: readonly { kind: unknown; len: unknown }[] }).chunks;
            const expanded: DiffRow[] = [];
            let shaped = true;
            for (const chunk of chunks) {
              if (
                typeof chunk.len !== "number" ||
                (chunk.kind !== "equal" &&
                  chunk.kind !== "changed" &&
                  chunk.kind !== "leftOnly" &&
                  chunk.kind !== "rightOnly")
              ) {
                shaped = false;
                break;
              }
              const kind = textCompareKindToDiffKind(chunk.kind as TextCompareRowKind);
              for (let i = 0; i < chunk.len; i += 1) expanded.push({ kind });
            }
            if (shaped && expanded.length > 0) {
              if (!cancelled) {
                setRulerRows(expanded);
                setRulerActive(0);
              }
              return;
            }
          }
        } catch {
          // Fall through to row paging below.
        }
        if (cancelled) return;
        const all: DiffRow[] = [];
        const PAGE = 200;
        let start = 0;
        while (!cancelled && start < total) {
          const page = await textGetRows(summary.session, start, PAGE);
          const rows: readonly TextRowDto[] = Array.isArray(page?.rows) ? page.rows : [];
          if (rows.length === 0) break;
          for (const row of rows) all.push({ kind: textCompareKindToDiffKind(row.kind) });
          start += rows.length;
        }
        if (!cancelled) {
          setRulerRows(all);
          setRulerActive(0);
        }
      } catch {
        if (!cancelled) setRulerRows([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [viewKind, activePaths.left, activePaths.right, compareTick, activeTabId]);

  const handleClearDemo = useCallback(() => {
    const id = demoId;
    setDemoId(null);
    setDemoDismissed(true);
    if (id !== null) {
      setTabPaths((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      // Wave-33: drop the tab's last-run snapshot with its paths so a
      // later tab never inherits a stale clean mark.
      setComparedPaths((prev) => {
        if (!(id in prev)) return prev;
        const next = { ...prev };
        delete next[id];
        return next;
      });
      void closeSession(id).then(() => {
        setStatusMessage("Blank session — pick your own files to start");
      });
    } else {
      setStatusMessage("Blank session — pick your own files to start");
    }
  }, [demoId, closeSession]);

  // Wave-117: shared compare runner. The toolbar Compare and pane drops
  // run the SAME kind-matched IPC path (no new IPC); drops pass the
  // just-set paths explicitly so the run never reads stale state.
  const runComparePaths = useCallback(
    async (
      kind: string,
      left: string,
      right: string,
      runId: string,
      runName: string,
      runPaths: TabPaths,
    ): Promise<boolean> => {
      if (!left || !right || !canCompareKind(kind)) return false;
      // Wave-140: test hook — real compare runs count (spec asserts
      // dual-drop fires exactly one compare, no double-compare).
      try {
        const w = window as unknown as Record<string, unknown>;
        w["__AC_COMPARE_COUNT__"] = (Number(w["__AC_COMPARE_COUNT__"]) || 0) + 1;
      } catch {
        // Best-effort only.
      }
      try {
        switch (kind) {
          case "text":
            await textCompare(left, right);
            break;
          case "folder":
            await folderCompare(left, right);
            break;
          case "hex":
            await hexCompare(left, right);
            break;
          case "table":
            await tableCompare(left, right, false);
            break;
          case "media":
            await pictureCompare(left, right);
            break;
          case "sync":
            await syncPlan(left, right);
            break;
          default:
            return false;
        }
        setActionError(null);
        // Wave-33: snapshot the inputs this run applied — the run tab is
        // clean until its inputs change again. Failures skip this, so a
        // failed run honestly keeps the dot.
        setComparedPaths((prev) => ({ ...prev, [runId]: runPaths }));
        setCompareTick((t) => t + 1);
        setStatusMessage(`Compare finished: ${runName}`);
        return true;
      } catch (e: unknown) {
        setActionError(e instanceof Error ? e.message : String(e));
        return false;
      }
    },
    [],
  );

  const handleCompare = useCallback(() => {
    if (!activeSession) return;
    const { left, right } = activePaths;
    // Wave-33: capture the run identity up front so the last-run
    // snapshot below lands on the tab that was actually compared,
    // even if the user switches tabs mid-run.
    const runId = activeSession.id;
    const runPaths = activePaths;
    void runComparePaths(activeSession.kind, left, right, runId, activeSession.name, runPaths);
  }, [activeSession, activePaths, runComparePaths]);

  // Wave-123: PathBar Enter submit (prop only; submit behavior stays in
  // PathBar, wave-124). Stages the edited side in `tabPaths` then runs
  // the SAME `runComparePaths` as toolbar Compare. Needs-two-paths and
  // non-comparable kinds stage honestly without firing.
  // Wave-134: side `"base"` re-opens the session carrying the new ancestor
  // (focus-existing dedups identical triples; merge has no compare run).
  const handlePathSubmit = useCallback(
    (side: "left" | "right" | "base", path: string) => {
      if (!activeSession) {
        setStatusMessage("Submit needs an open session");
        return;
      }
      if (side === "base") {
        void handleOpenSession(
          activeSession.kind,
          activePaths.left || null,
          activePaths.right || null,
          path || null,
        );
        return;
      }
      const runId = activeSession.id;
      const live = tabPathsRef.current[runId] ?? activePaths;
      const next: TabPaths = {
        ...live,
        kind: activeSession.kind,
        left: side === "left" ? path : live.left,
        right: side === "right" ? path : live.right,
      };
      tabPathsRef.current = { ...tabPathsRef.current, [runId]: next };
      setTabPaths(tabPathsRef.current);
      if (!next.left || !next.right) {
        setStatusMessage(`Path set: ${path} — needs two paths`);
        return;
      }
      if (!canCompareKind(next.kind)) {
        setStatusMessage(`Path set: ${path} (no compare for kind "${next.kind}")`);
        return;
      }
      void runComparePaths(next.kind, next.left, next.right, runId, activeSession.name, next);
    },
    [activeSession, activePaths, runComparePaths],
  );

  // Wave-117: pane file drop (Tauri BC parity). A Finder/Explorer drop
  // onto the left/right half sets that side's path in `tabPaths` and
  // auto-compares through the SAME `runComparePaths` path as the toolbar
  // Compare (no new IPC). One-sided drops only stage the path and say
  // which side is still missing; non-comparable kinds stage honestly
  // without firing.
  //
  // Wave-120: BC `DIRVIEWNEW` / `FILEVIEWNEW` auto-switch (verify-first vs
  // observer sections 4-6). A folderish drop (trailing slash or
  // `.zip`/`.tar`/`.gz` archive — the shared `isFolderishPath`
  // classifier) in a file-kind session opens a FOLDER session instead of
  // staging a file path; a file drop in a folder-kind session opens a
  // TEXT session. The other side carries over only when it already fits
  // the target class, else it stays blank with a `drop the other side`
  // note. `merge`/`other` kinds never switch (honest fallback: 117 stage
  // path). Same-class single drops keep the exact 117 behavior below.
  const handlePaneDrop = useCallback(
    (side: PaneSide, path: string) => {
      if (!activeSession) {
        setStatusMessage("Drop needs an open session");
        return;
      }
      const cls = dropKindClass(activeSession.kind);
      const folderish = isFolderishPath(path);
      if ((folderish && cls === "file") || (!folderish && cls === "folder")) {
        const targetKind = folderish ? "folder" : "text";
        const carry = (p: string): string => {
          if (p === "") return "";
          return isFolderishPath(p) === folderish ? p : "";
        };
        const left = side === "left" ? path : carry(activePaths.left);
        const right = side === "right" ? path : carry(activePaths.right);
        const switchNote =
          targetKind === "folder"
            ? "Folder dropped — opened folder compare (BC DIRVIEWNEW)"
            : "File dropped — opened text compare (BC FILEVIEWNEW)";
        void (async () => {
          const newId = await handleOpenSession(targetKind, left, right);
          if (!newId) {
            setStatusMessage("Drop needs an open session");
            return;
          }
          if (left !== "" && right !== "" && canCompareKind(targetKind)) {
            void runComparePaths(targetKind, left, right, newId, `${targetKind} session`, {
              kind: targetKind,
              left,
              right,
            });
            return;
          }
          setStatusMessage(`${switchNote}, drop the other side to compare`);
        })();
        return;
      }
      const runId = activeSession.id;
      const next: TabPaths = {
        kind: activeSession.kind,
        left: side === "left" ? path : activePaths.left,
        right: side === "right" ? path : activePaths.right,
      };
      setTabPaths((prev) => ({ ...prev, [runId]: next }));
      const label = side === "left" ? "Left" : "Right";
      if (!next.left || !next.right) {
        setStatusMessage(`${label} set: ${path} — drop the other side to compare`);
        return;
      }
      if (!canCompareKind(activeSession.kind)) {
        setStatusMessage(`${label} set: ${path} (no auto compare for kind "${activeSession.kind}")`);
        return;
      }
      void runComparePaths(activeSession.kind, next.left, next.right, runId, activeSession.name, next);
    },
    [activeSession, activePaths, runComparePaths, handleOpenSession],
  );

  // Wave-120: dual-file drop (BC parity: Item 0 -> Left, Item 1 -> Right in
  // ONE drop, regardless of which half it lands on). Same-class pairs
  // stage both sides and auto-compare through `runComparePaths` (no new
  // IPC); cross-class pairs auto-switch the session kind first
  // (`DIRVIEWNEW` when any dropped path is folderish in a file-kind
  // session, `FILEVIEWNEW` when all dropped paths are files in a
  // folder-kind session, same rule for `other` kinds as an honest
  // default). Extra paths past the second are ignored (two panes only).
  const handlePaneDropPaths = useCallback(
    (paths: readonly string[], _side: PaneSide) => {
      if (!activeSession) {
        setStatusMessage("Drop needs an open session");
        return;
      }
      const first = paths[0];
      const second = paths[1];
      if (first === undefined || second === undefined) return;
      const cls = dropKindClass(activeSession.kind);
      const anyFolderish = isFolderishPath(first) || isFolderishPath(second);
      const allFiles = !isFolderishPath(first) && !isFolderishPath(second);
      const targetKind =
        cls === "file" && anyFolderish
          ? "folder"
          : cls === "folder" && allFiles
            ? "text"
            : cls === "other"
              ? anyFolderish
                ? "folder"
                : "text"
              : activeSession.kind;
      if (targetKind !== activeSession.kind) {
        void (async () => {
          const newId = await handleOpenSession(targetKind, first, second);
          if (!newId) {
            setStatusMessage("Drop needs an open session");
            return;
          }
          if (canCompareKind(targetKind)) {
            void runComparePaths(targetKind, first, second, newId, `${targetKind} session`, {
              kind: targetKind,
              left: first,
              right: second,
            });
          }
        })();
        return;
      }
      const runId = activeSession.id;
      const next: TabPaths = { kind: activeSession.kind, left: first, right: second };
      setTabPaths((prev) => ({ ...prev, [runId]: next }));
      if (!canCompareKind(activeSession.kind)) {
        setStatusMessage(`2 files set (no auto compare for kind "${activeSession.kind}")`);
        return;
      }
      void runComparePaths(activeSession.kind, next.left, next.right, runId, activeSession.name, next);
    },
    [activeSession, runComparePaths, handleOpenSession],
  );

  // Wave-117: production-shell Tauri drag-drop subscription (no-op under
  // `VITE_MOCK_IPC=1` / plain browser, where the HTML5 handlers own drops).
  // Wave-120: dual-drop paths ride the same hook (third arg).
  useTauriPaneDrop(viewKind !== null, handlePaneDrop, handlePaneDropPaths);

  const handleSync = useCallback(() => {
    void (async () => {
      if (!activeSession) return;
      const { left, right } = activePaths;
      if (!left || !right) return;
      try {
        const plan = await syncPlan(left, right);
        const steps = Array.isArray(plan) ? 0 : ((plan.steps?.length ?? 0) as number);
        setActionError(null);
        setStatusMessage(`Sync plan: ${steps} step(s) for ${activeSession.name}`);
      } catch (e: unknown) {
        setActionError(e instanceof Error ? e.message : String(e));
      }
    })();
  }, [activeSession, activePaths]);

  // Wave-13: ruler click/drag/keyboard jumps the virtualized text pane.
  // Wave-14: pending programmatic jump target. `handleRulerJump` sets
  // `scrollTop` from a row fraction, but browsers snap `scrollTop` to
  // whole pixels, so reading the position back via `floor(frac * total)`
  // can land one row low (jump to 6 reads back as 5) and the next hunk
  // jump re-targets the row it just left. The scroll listener below
  // honors a fresh pending target instead of recomputing, so jumps are
  // authoritative; genuine user scrolls (no fresh pending jump) still
  // derive the position from the live fraction.
  const jumpRef = useRef<{ readonly target: number; readonly at: number } | null>(null);

  // The virtualized pane scrolls at fixed ROW_PX offsets, so
  // rowIndexToScrollTop lands exactly; short diffs render
  // unvirtualized and fall back to scrolling the target row into view.
  const handleRulerJump = useCallback(
    (index: number) => {
      const total = rulerRows.length;
      if (total === 0) return;
      const target = Math.min(total - 1, Math.max(0, index));
      setRulerActive(target);
      const scroller = document.querySelector('[data-testid="textview/virtual"]');
      if (scroller instanceof HTMLElement) {
        jumpRef.current = { target, at: Date.now() };
        scroller.scrollTop = rowIndexToScrollTop(
          target,
          total,
          scroller.scrollHeight,
          scroller.clientHeight,
        );
        return;
      }
      document
        .querySelector(`[data-testid="textview/row-${target}"]`)
        ?.scrollIntoView({ block: "start" });
    },
    [rulerRows.length],
  );

  // Wave-13: track the visible top row (capture phase catches the
  // virtual scroller) so the ruler highlights the live position.
  useEffect(() => {
    if (rulerRows.length === 0) return;
    const total = rulerRows.length;
    const onScroll = (e: Event): void => {
      const scroller = document.querySelector('[data-testid="textview/virtual"]');
      if (!(scroller instanceof HTMLElement)) return;
      // Ignore scrolls from outside the text pane (the capture listener
      // sees every scrollable on the page, e.g. dialogs).
      const fromPane =
        e.target === scroller ||
        (e.target instanceof HTMLElement &&
          e.target.closest('[data-testid="textview/virtual"]') !== null);
      if (!fromPane) return;
      const pending = jumpRef.current;
      if (pending !== null && Date.now() - pending.at < 1000) {
        jumpRef.current = null;
        setRulerActive(Math.min(total - 1, Math.max(0, pending.target)));
        return;
      }
      jumpRef.current = null;
      const span = scroller.scrollHeight - scroller.clientHeight;
      if (span > 0) {
        const frac = Math.min(1, Math.max(0, scroller.scrollTop / span));
        setRulerActive(Math.min(total - 1, Math.floor(frac * total + 1e-6)));
      } else {
        setRulerActive(0);
      }
    };
    document.addEventListener("scroll", onScroll, true);
    return () => document.removeEventListener("scroll", onScroll, true);
  }, [rulerRows.length, activeTabId]);

  const hasPaths = Boolean(activeSession) && activePaths.left !== "" && activePaths.right !== "";
  // Wave-151: half-staged sessions mount the view (one-sided drop stage:
  // ek side set + dusri blank par bhi `view-{kind}` + pathbar + stage note
  // dikhte hain, bina compare (Compare parked); fully-blank sessions keep
  // the 138 `blank-empty`, so TextView demo auto-load kabhi fire nahi hota.
  const hasAnyPath = Boolean(activeSession) && (activePaths.left !== "" || activePaths.right !== "");
  const sessionKind: string | null = activeSession?.kind ?? null;
  const compareBlockedReason = sessionKind === null
    ? "needs an open session"
    : !hasPaths
      ? "needs two paths"
      : sessionKind === "merge"
        ? "merge uses the Merge view"
        : !canCompareKind(sessionKind)
          ? `no compare for kind "${sessionKind}"`
          : null;
  const syncEnabled =
    sessionKind !== null &&
    hasPaths &&
    (sessionKind === "folder" || sessionKind === "sync");

  // Wave-14: hunk-start indices from rulerRows ONLY. A hunk starts at
  // each index where kind != "same" and the previous row is "same"
  // (or it is index 0). Toolbar Next/Prev jumps wrap around and reuse
  // handleRulerJump so the pane scrolls and rulerActive follows.
  const hunkStarts = React.useMemo(() => {
    const starts: number[] = [];
    for (let i = 0; i < rulerRows.length; i += 1) {
      if (rulerRows[i]?.kind === "same") continue;
      if (i === 0 || rulerRows[i - 1]?.kind === "same") starts.push(i);
    }
    return starts;
  }, [rulerRows]);

  const handleNextHunk = useCallback(() => {
    if (viewKind !== "text" || hunkStarts.length === 0) return;
    const next = hunkStarts.find((s) => s > rulerActive) ?? hunkStarts[0];
    if (next !== undefined) handleRulerJump(next);
  }, [viewKind, hunkStarts, rulerActive, handleRulerJump]);

  const handlePrevHunk = useCallback(() => {
    if (viewKind !== "text" || hunkStarts.length === 0) return;
    let prev = hunkStarts[hunkStarts.length - 1];
    for (let i = hunkStarts.length - 1; i >= 0; i -= 1) {
      const s = hunkStarts[i];
      if (s !== undefined && s < rulerActive) {
        prev = s;
        break;
      }
    }
    if (prev !== undefined) handleRulerJump(prev);
  }, [viewKind, hunkStarts, rulerActive, handleRulerJump]);

  const hunkNavEnabled = viewKind === "text" && hunkStarts.length > 0;

  // Wave-27: live statusbar diff stats (Beyond Compare style bottom
  // summary). Derived from the ALREADY-LOADED `rulerRows` for the active
  // text view (same `same`/`del`/`ins`/`chg` mapping the pane paints, so
  // counts line up 1:1 with visible rows); no new IPC. Non-text kinds
  // and empty states resolve `undefined` so the bar honestly clears to
  // the `Ready` default instead of showing stale counts. Gated on
  // `viewKind` (not just row presence) so switching sessions/tabs clears
  // immediately, before the ruler effect reloads.
  const diffStats: DiffStats | undefined = React.useMemo(() => {
    if (viewKind !== "text" || rulerRows.length === 0) return undefined;
    let added = 0;
    let removed = 0;
    let changed = 0;
    for (const row of rulerRows) {
      if (row.kind === "ins") added += 1;
      else if (row.kind === "del") removed += 1;
      else if (row.kind === "chg") changed += 1;
    }
    return { added, removed, changed, total: rulerRows.length };
  }, [viewKind, rulerRows]);

  // Wave-125: toolbar Copy/Merge live (Beyond Compare parity).
  // Copy rides the TextView gutter path through the
  // `autocompare:text-copy` window event (same
  // text_copy_section + clipboard + text_apply_edit + text_save_side
  // flow as the gutter arrows and the `l` / `r` keys). Enabled only
  // when a text row inside a hunk is selected; the take side follows
  // the selection (left content present = take left, else take right).
  // Merge is enabled only in merge-kind sessions and jumps to the
  // Merge view (scroll + status); every other kind keeps the honest
  // parked label (blank keeps the legacy `select a row` / `Merge view`
  // substrings the chrome specs pin).
  const copyTakeSide: TextSide | null =
    viewKind === "text" && selectedRow !== null
      ? selectedRow.left !== null
        ? "left"
        : "right"
      : null;
  const copyInHunk = copyTakeSide !== null && selectedRow !== null && selectedRow.kind !== "equal";
  const mergeLive = sessionKind === "merge";

  const handleToolbarCopy = useCallback(() => {
    if (!copyInHunk || selectedIndex === null || copyTakeSide === null) return;
    window.dispatchEvent(
      new CustomEvent("autocompare:text-copy", { detail: { index: selectedIndex, dir: copyTakeSide } }),
    );
    setStatusMessage(`Copy take-${copyTakeSide} requested for row ${selectedIndex}`);
  }, [copyInHunk, selectedIndex, copyTakeSide]);

  // Wave-136: BC-style Swap sides (active tab left<->right exchange).
  // Touches ONLY the active tabPaths record (base ancestor untouched);
  // never snapshots, saves, or dialogs, so the tab dirty state rides
  // through unchanged. Views reload from the swapped paths; blank
  // tabs are a no-op, never an error.
  const handleSwapSides = useCallback(() => {
    const id = activeTabId;
    if (id === WELCOME_TAB.id || !activeSession || activeSession.id !== id) return;
    setTabPaths((prev) => {
      const cur = prev[id];
      if (!cur || (cur.left === "" && cur.right === "")) return prev;
      return { ...prev, [id]: { ...cur, left: cur.right, right: cur.left } };
    });
    setStatusMessage("Swapped left and right");
  }, [activeTabId, activeSession]);

  const handleMergeJump = useCallback(() => {
    if (!mergeLive) return;
    document.querySelector('[data-testid="view-merge"]')?.scrollIntoView({ block: "start" });
    const focusable = document.querySelector(
      '[data-testid="view-merge"] button:not([disabled]), [data-testid="view-merge"] [tabindex]',
    );
    if (focusable instanceof HTMLElement) focusable.focus();
    setStatusMessage("Merge view — resolve conflicts below");
  }, [mergeLive]);

  const toolbarActions: readonly ToolbarAction[] = [
    { id: "open", icon: FolderOpen, label: "Open session", enabled: true, onClick: () => setManagerOpen(true) },
    {
      id: "compare",
      icon: Columns2,
      label: compareBlockedReason ? `Compare (${compareBlockedReason})` : "Compare",
      enabled: compareBlockedReason === null,
      onClick: handleCompare,
    },
    {
      // Wave-136: BC Swap sides (active tab left<->right, base untouched).
      id: "swap",
      icon: ArrowLeftRight,
      label: "Swap sides",
      enabled: activeSession !== null,
      disabledReason: activeSession !== null ? null : "needs an open session",
      onClick: handleSwapSides,
    },
    {
      id: "prev-hunk",
      icon: ChevronUp,
      label: hunkNavEnabled ? "Previous hunk" : "Previous hunk (no hunks)",
      enabled: hunkNavEnabled,
      onClick: handlePrevHunk,
    },
    {
      id: "next-hunk",
      icon: ChevronDown,
      label: hunkNavEnabled ? "Next hunk" : "Next hunk (no hunks)",
      enabled: hunkNavEnabled,
      onClick: handleNextHunk,
    },
    {
      // Wave-125: live in text kind with a hunk row selected (same
      // path as the gutter arrows); parked with an honest reason
      // otherwise.
      id: "copy",
      icon: Copy,
      label: copyInHunk
        ? `Copy (take ${copyTakeSide})`
        : viewKind === "text" && selectedRow !== null
          ? "Copy (selected row is not a difference)"
          : "Copy (select a row first)",
      enabled: copyInHunk,
      disabledReason: copyInHunk
        ? null
        : viewKind === "text" && selectedRow !== null
          ? "selected row is not a difference"
          : "select a row first",
      onClick: handleToolbarCopy,
    },
    {
      id: "sync",
      icon: RefreshCw,
      label: syncEnabled ? "Sync" : "Sync (needs folder session)",
      enabled: syncEnabled,
      onClick: handleSync,
    },
    {
      // Wave-125: live in merge-kind sessions (jumps to the Merge
      // view); parked with an honest reason otherwise.
      id: "merge",
      icon: GitMerge,
      label: mergeLive ? "Merge" : "Merge (use Merge view)",
      enabled: mergeLive,
      disabledReason: mergeLive ? null : "use Merge view",
      onClick: handleMergeJump,
    },
  ];

  const inlineError = actionError ?? sessionError;
  // Wave-10: demo banner shows only on the auto-opened demo tab.
  const isDemo = demoId !== null && activeSession?.id === demoId;

  return (
    <DiffStatsContext.Provider value={diffStats}>
      <ChromeShell
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={handleSelectTab}
        onCloseTab={handleCloseTab}
        onNewTab={handleNewTab}
        toolbar={{ actions: toolbarActions }}
        leftPath={activePaths.left}
        rightPath={activePaths.right}
        onSubmitPath={handlePathSubmit}
        basePath={activePaths.base ?? ""}
        showBase={sessionKind === "merge"}
        // Wave-140: PathBar/strip dual-drop -> single auto-compare
        // (BC Item0 -> left, Item1 -> right; side arg ignored for dual).
        onDropPaths={(paths) => handlePaneDropPaths(paths, "left")}
        status={{
          left: activeSession ? `session: ${activeSession.name}` : "session: none",
          right: activeSession
            ? `${activeSession.kind} \u2022 ${activeSession.pageCount} tab(s)`
            : "0 files",
        }}
        statusMessage={statusMessage}
        diffRows={rulerRows}
        onJump={handleRulerJump}
        activeIndex={rulerActive}
        detail={viewKind === "text" ? <LineDetailPane row={selectedRow} /> : null}
      >
        {inlineError && (
          <div role="alert" data-testid="workspace/error" className="mx-auto mt-4 max-w-3xl rounded-md border border-[var(--ac-alert-border)] bg-[var(--ac-alert-bg)] px-4 py-2 text-xs font-medium text-[var(--ac-alert-fg)] shadow-xs">
            <span>{inlineError}</span>{" "}
            <button
              type="button"
              aria-label="Dismiss error"
              onClick={() => {
                setActionError(null);
                clearError();
              }}
              className="underline hover:no-underline focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              Dismiss
            </button>
          </div>
        )}
        {!activeSession ? (
          // Wave-74: Home is two-pane (Beyond Compare Home style) — the
          // `HomeSessionsPane` left list renders from the existing
          // `visibleSessions`/`activeSession` data (title + kind + dirty
          // dot, click activates via `handleSelectTab`). NO new session
          // kinds, NO backend change, NO new HOME_CARDS: the right pane
          // below is the untouched cards grid with all legacy testids.
          <div
            className="mx-auto flex w-full max-w-4xl flex-row items-start gap-6 px-8 py-16"
            data-testid="workspace/welcome"
          >
            <HomeSessionsPane
              sessions={visibleSessions.map((s) => ({
                id: s.id,
                title: s.name,
                kind: s.kind,
                dirty: isTabDirty(s.id),
              }))}
              activeId={null} // Welcome branch: no session is active (Home itself is selected)
              onSelect={handleSelectTab}
            />
            <div className="flex min-w-0 flex-1 flex-col items-center gap-6 text-center">
              <h1 className="ac-display font-semibold text-[var(--ac-fg)]">AutoCompare</h1>
            <p className="ac-body text-[var(--ac-fg-muted)]">
              Open or create a session to start comparing.
            </p>
            <button
              type="button"
              data-testid="workspace/open-manager"
              onClick={() => setManagerOpen(true)}
              className="rounded-lg bg-[var(--ac-accent)] px-4 py-2 ac-body font-medium text-white shadow-xs transition hover:opacity-90 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              Open Session Manager
            </button>
            <div
              className="grid w-full max-w-2xl grid-cols-2 gap-3 text-left sm:grid-cols-3"
              aria-label="Start a new comparison"
            >
              {HOME_CARDS.map((card) => {
                const CardIcon = card.icon;
                return (
                  <button
                    key={card.kind}
                    type="button"
                    data-testid={`home/new-${card.kind}`}
                    title={`${card.label} — ${card.hint}`}
                    onClick={() => void handleOpenSession(card.kind)}
                    className="flex items-center gap-3 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-4 py-3 transition-colors hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                  >
                    <CardIcon size={20} aria-hidden="true" className="shrink-0 text-[var(--ac-fg-muted)]" />
                    <span className="min-w-0">
                      <span className="ac-chrome-label block truncate font-medium text-[var(--ac-fg)]">
                        {card.label}
                      </span>
                      <span className="ac-meta block truncate text-[var(--ac-fg-muted)]">
                        {card.hint}
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
            <p className="ac-mono text-xs text-[var(--ac-fg-muted)]" data-testid="session-count">
              sessions: {visibleSessions.length}
            </p>
            </div>
          </div>
        ) : !hasAnyPath ? (
          // Wave-138: blank-path session honest empty (views ke 130-empty
          // guards parity: do paths mango, koi compare/error/demo nahi).
          // ViewRouter mount nahi hota isliye TextView ka demo auto-load
          // kabhi fire nahi hota; paths bharte hi normal compare resume.
          // Wave-151: sirf fully-blank yahan aata hai; half-staged (ek
          // side set) neeche view mount karta hai taaki one-sided drop
          // stage + auto-switch stage note dikhe bina compare.
          <div
            className="mx-auto flex w-full max-w-3xl flex-col items-center gap-3 px-8 py-16 text-center"
            data-testid="workspace/blank-empty"
          >
            <p className="ac-body text-[var(--ac-fg-muted)]">
              Pick two paths to compare — needs two paths.
            </p>
            <p className="ac-mono text-xs text-[var(--ac-fg-muted)]" data-testid="session-count">
              sessions: {visibleSessions.length}
            </p>
          </div>
        ) : viewKind ? (
          <PaneDropZone
            enabled={viewKind !== null}
            onDropPath={handlePaneDrop}
            onDropPaths={handlePaneDropPaths}
            className="flex min-h-0 flex-1 flex-col"
          >
            {isDemo && (
              <div
                role="status"
                data-testid="workspace/demo-banner"
                className="mx-4 mt-2 flex items-center justify-between gap-3 rounded-lg border border-[var(--ac-border)] bg-[var(--ac-surface-secondary)] px-4 py-2 text-xs font-medium text-[var(--ac-fg)]"
              >
                <span>Demo — pick your own files to start</span>
                <button
                  type="button"
                  data-testid="workspace/demo-clear"
                  onClick={handleClearDemo}
                  className="rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg)] px-2.5 py-1 text-xs font-medium text-[var(--ac-fg-muted)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
                >
                  Start blank
                </button>
              </div>
            )}
            <p className="ac-mono px-4 pt-2 text-[11px] text-[var(--ac-fg-muted)]" data-testid="session-count">
              sessions: {visibleSessions.length}
            </p>
            <ViewRouter
              key={`${activeTabId}:${compareTick}`}
              kind={viewKind}
              leftPath={activePaths.left}
              rightPath={activePaths.right}
              basePath={activePaths.base}
              sessionId={activeSession.id}
              selectedIndex={selectedIndex}
              onSelectRow={handleSelectRow}
              onOpenPaths={handleOpenPaths}
            />
          </PaneDropZone>
        ) : (
          <div className="mx-auto flex max-w-3xl flex-col items-center gap-6 px-8 py-16 text-center" data-testid="view-router">
            <h1 className="ac-display font-semibold text-[var(--ac-fg)]">AutoCompare</h1>
            <p className="ac-body text-[var(--ac-fg-muted)]">
              No data view for kind &ldquo;{activeSession.kind}&rdquo; yet. Manage paths in the Session Manager.
            </p>
            <p className="ac-mono text-xs text-[var(--ac-fg-muted)]" data-testid="session-count">
              sessions: {visibleSessions.length}
            </p>
          </div>
        )}
      </ChromeShell>
      <SessionManager
        isOpen={managerOpen}
        onClose={() => setManagerOpen(false)}
        sessions={sessions}
        activeId={activeId}
        cleanupCount={cleanupCount}
        onChange={handleManagerChange}
        onOpenSession={handleOpenSession}
        onSaveSession={handleSaveSession}
        onLoadSession={handleLoadSession}
        onDeleteSession={deleteSession}
      />
    </DiffStatsContext.Provider>
  );
}
