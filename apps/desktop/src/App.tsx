import React, { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlignJustify, ArrowLeftRight, Binary, ChevronDown, ChevronUp, Code, Columns2, Copy, Equal, FileText, Filter, Folder, FolderOpen, GitMerge, Home, Image, List, Minus, RefreshCw, Table, Type } from "lucide-react";
import { ChromeShell, rowIndexToScrollTop, textCompareKindToDiffKind } from "./components/chrome";
import { Gallery } from "./components/ui/Gallery";
import {
  dispatchViewCommand,
  dispatchViewFilter,
  dispatchViewMinor,
} from "./components/chrome";
import { dispatchViewFormat } from "./components/views/text/controls/FormatMenu";
// Wave-201: app-root font wiring (typography ONLY). Owned polish sheet:
// Inter/system-ui + JetBrains Mono/ui-monospace stacks, soft inline
// accent, toolbar surfaces, hairlines — all per existing --ac-* tokens.
import "./styles/typography-201.css";
import { dispatchViewRules } from "./components/views/folder/controls/RulesPanel";
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
import { mergeTextPlanForTriple } from "./ipc/merge";
import { pictureCompare } from "./ipc/media";
import { HomeSessionsPane } from "./components/home/controls";
import { PaneDropZone, isFolderishPath, type PaneSide } from "./components/dnd";
import { useNativeFileDrop } from "./components/chrome/useNativeFileDrop";
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
  { kind: "folder", icon: Folder, label: "Folder Compare", hint: "Directories & archives" },
  { kind: "hex", icon: Binary, label: "Hex Compare", hint: "Bytes and offsets" },
  { kind: "table", icon: Table, label: "Table Compare", hint: "Rows and columns" },
  { kind: "merge", icon: GitMerge, label: "Merge", hint: "Three-way merge" },
  { kind: "sync", icon: RefreshCw, label: "Sync", hint: "Mirror folders" },
  { kind: "media", icon: Image, label: "Media Compare", hint: "Picture and audio" },
] as const;

/**
 * Wave-D3: transient status notes fade. Event-style notes ("Tab closed: …",
 * "Tab selected: …", staged-path notes, Take/Copy takes, …) revert to the
 * honest "Ready" default after a quiet window so the bar never shows a
 * stale event next to fresh content. Persistent outcomes the specs pin
 * ("Compare finished", "Sync plan", "Session saved", "Nothing to save",
 * errors) are NOT matched — they stay until the next action replaces them.
 */
const TRANSIENT_STATUS_RE =
  /^(Tab (closed|selected)|Session (active|opened|loaded)|Demo session ready|Blank session|Swapped left and right|Welcome|Session Manager opened.*|Find (focused|is not available)|Take |Copy take-|Merge take-|Merge view|Path set|Left set|Right set|Drop needs|2 files set|Submit needs|Theme follows|Layout toggle|Settings UI|.*drop the other side.*|.*no compare for kind.*|.*opened (folder|text) compare.*)$/;

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

/** Kinds whose toolbar-level Compare maps 1:1 to a compare IPC (merge
 * runs `merge_text_plan` over the staged triple; the MergeView re-plans
 * from the same triple, so Compare/dual-drop/auto-compare stay in sync). */
function canCompareKind(kind: string): boolean {
  return kind === "text" || kind === "folder" || kind === "hex" || kind === "table" || kind === "media" || kind === "sync" || kind === "merge";
}

const COPY_STORE_PREFIX = "ac:textcopy125:";

function copyStoreKey(left: string, right: string): string {
  return `${COPY_STORE_PREFIX}${left}\u0000${right}`;
}

interface StoredHunkCopy {
  readonly start: number;
  readonly end: number;
  readonly dir: TextSide;
}

function readStoredCopies(left: string, right: string): StoredHunkCopy[] {
  try {
    if (typeof window === "undefined" || !window.localStorage) return [];
    const raw = window.localStorage.getItem(copyStoreKey(left, right));
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const out: StoredHunkCopy[] = [];
    for (const entry of parsed) {
      if (typeof entry === "object" && entry !== null) {
        const start = (entry as { start?: unknown }).start;
        const end = (entry as { end?: unknown }).end;
        const dir = (entry as { dir?: unknown }).dir;
        if (
          typeof start === "number" &&
          Number.isInteger(start) &&
          start >= 0 &&
          typeof end === "number" &&
          Number.isInteger(end) &&
          end >= start &&
          (dir === "left" || dir === "right")
        ) {
          out.push({ start, end, dir });
        }
      }
    }
    return out;
  } catch {
    return [];
  }
}

function applyStoredCopiesToDiffRows(
  rows: readonly DiffRow[],
  copies: readonly StoredHunkCopy[],
): DiffRow[] {
  if (copies.length === 0 || rows.length === 0) return [...rows];
  const next = [...rows];
  for (const copy of copies) {
    const lo = Math.max(0, copy.start);
    const hi = Math.min(next.length - 1, copy.end);
    for (let i = lo; i <= hi; i += 1) {
      next[i] = { kind: "same" };
    }
  }
  return next;
}

/**
 * Wave-L2: ViewCommandBus vocabulary (BC unified-toolbar parity).
 *
 * `ViewCommand` routes prev/next/copy/swap per active viewKind from ONE
 * place (`handleViewCommand` in `App` scope): text keeps the live hunk
 * handlers, every other kind gets an honest parked status note (per-view
 * wiring lands in L3) instead of a silent no-op. The top Toolbar calls
 * the bus directly; `TextToolbar` prefers its `onBusCommand` prop when
 * L3 passes it and falls back to the legacy TextView props meanwhile —
 * testids stay byte-identical either way.
 *
 * `ViewFilterKind` lifts All/Diffs/Same/Context into the unified top
 * Toolbar as filter actions dispatching the `autocompare:view-filter`
 * window event (`{ filter, viewKind }`); every view listens already, so
 * the top bar is the single dispatch point and the view-level controls
 * keep working untouched. Minor is live in text and table kinds (dispatches
 * `autocompare:view-minor` with the kind through the view bus, applied
 * through the text/table IPC ignore options). Wave-167: Rules/Format are live
 * top-bar actions dispatching `autocompare:view-rules` (folder) /
 * `autocompare:view-format` (text) to the in-view panels, honestly parked elsewhere.
 * Wave-169: Minor extends to table kind (dispatchViewMinor with kind table
 * through existing view bus + table IPC ignore flags; text behavior unchanged).
 */
export type ViewCommand =
  | "prev"
  | "next"
  | "copy"
  | "copy-right"
  | "swap"
  | "merge"
  | "take-left"
  | "take-right"
  | "take-both";
export type ViewFilterKind = "all" | "diffs" | "same" | "context";

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
  // Wave-D3: transient fade — event-style notes revert to "Ready" after a
  // quiet window (see TRANSIENT_STATUS_RE above); the text stays byte-same
  // until then so specs pinning last-state strings keep passing, and every
  // next action still replaces it immediately via the setters below.
  useEffect(() => {
    if (!TRANSIENT_STATUS_RE.test(statusMessage)) return;
    const timer = window.setTimeout(() => {
      setStatusMessage((current) => (TRANSIENT_STATUS_RE.test(current) ? "Ready" : current));
    }, 8000);
    return () => window.clearTimeout(timer);
  }, [statusMessage]);
  // Wave-201: app-root font wiring (typography ONLY). Bundled Inter /
  // JetBrains Mono stacks live in tokens.css; pin the root element to the
  // UI stack so first paint + portals ride it even before the body rule
  // applies. No DOM, testid, color, or geometry change.
  useEffect(() => {
    document.documentElement.style.setProperty("font-family", "var(--ac-font-ui)");
  }, []);
  const [actionError, setActionError] = useState<string | null>(null);
  const [compareTick, setCompareTick] = useState(0);
  // C1c: last real folder-compare session id. `runComparePaths` sets it
  // from the `folder_compare` receipt; the ChromeShell forwards it to the
  // Toolbar Stop path so `folder_cancel(session)` fires for real.
  const [folderSessionId, setFolderSessionId] = useState<string | null>(null);
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
  // Wave-G2: live mirror of hunk starts for the menu Take fallback
  // (`handleMenuTake` is defined above the `hunkStarts` memo, so it reads
  // the ref — fresh at call time, no stale closure).
  const hunkStartsRef = useRef<number[]>([]);
  // Wave-203: ViewCommandBus forward ref to avoid TDZ in menu handlers
  const handleViewCommandRef = useRef<(cmd: ViewCommand) => void>(() => {});

  const handleSelectRow = useCallback((index: number | null, row: TextRowDto | null) => {
    setSelectedIndex(index);
    setSelectedRow(row);
    if (index !== null) setRulerActive(index);
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
  const viewKindRef = useRef<DataViewKind | null>(viewKind);
  useEffect(() => {
    viewKindRef.current = viewKind;
  });

  const handleSelectTab = useCallback(
    (id: string) => {
      if (id === WELCOME_TAB.id && visibleSessions.length === 0) {
        setStatusMessage("Welcome");
        return;
      }
      // H1: Home par debug-copy nahi — `Tab selected: home` ki jagah
      // honest `Ready` (BC jaisa neutral). Comparison tabs ka message
      // byte-identical rehta hai.
      if (id === WELCOME_TAB.id) {
        selectSession(id);
        setStatusMessage("Ready");
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
  // Wave-G2: a selected equal row falls back to the nearest hunk (next
  // hunk at/after the selection, else the last) so menu Take still
  // merges; with NO selection at all the honest parked note stays
  // (menumerge-141 pin).
  // Wave-E1: when merge view active (`viewKind === "merge"`), route
  // Cmd+D/E and menu Take directly to MergeView take-left / take-right /
  // take-both logic instead of `autocompare:text-copy`. TextView keeps
  // its exact text path intact.
  const handleMenuTake = useCallback(
    (dir: TextSide | "both") => {
      const activeKind = viewKindRef.current ?? viewKind;
      if (activeKind === "merge") {
        dispatchViewCommand("copy", "merge", { dir });
        setStatusMessage(`Copy take-${dir} requested (merge)`);
        return;
      }
      if (dir === "both") {
        setStatusMessage("Take both is only available in Merge view");
        return;
      }
      const isHunkRow =
        selectedIndex !== null &&
        ((selectedRow !== null && selectedRow.kind !== "equal") ||
          (selectedRow === null &&
            rulerRows[selectedIndex] !== undefined &&
            rulerRows[selectedIndex]?.kind !== "same"));
      if (isHunkRow && selectedIndex !== null) {
        window.dispatchEvent(
          new CustomEvent("autocompare:text-copy", { detail: { index: selectedIndex, dir } }),
        );
        setStatusMessage(`Copy take-${dir} requested for row ${selectedIndex}`);
        return;
      }
      if (selectedIndex !== null) {
        const starts =
          hunkStartsRef.current.length > 0
            ? hunkStartsRef.current
            : (() => {
                const s: number[] = [];
                for (let i = 0; i < rulerRows.length; i += 1) {
                  if (rulerRows[i]?.kind === "same") continue;
                  if (i === 0 || rulerRows[i - 1]?.kind === "same") s.push(i);
                }
                return s;
              })();
        if (starts.length > 0) {
          const atOrAfter = starts.find((s) => s >= selectedIndex);
          const target = atOrAfter ?? starts[starts.length - 1]!;
          window.dispatchEvent(
            new CustomEvent("autocompare:text-copy", { detail: { index: target, dir } }),
          );
          setStatusMessage(`Copy take-${dir} requested for row ${target}`);
          return;
        }
      }
      setStatusMessage(`Take ${dir} needs a hunk row selected`);
    },
    [viewKind, selectedIndex, selectedRow, rulerRows],
  );

  // Wave-E1: listen for merge take outcome from MergeView and update statusbar
  useEffect(() => {
    const handleMergeTakeResult = (event: Event) => {
      const customEvent = event as CustomEvent<{
        dir: string;
        conflictId?: number;
        ok: boolean;
        reason?: string;
      }>;
      if (customEvent.detail) {
        if (customEvent.detail.ok) {
          const id = customEvent.detail.conflictId;
          const dir = customEvent.detail.dir;
          setStatusMessage(`Merge take-${dir} applied for conflict ${id}`);
        } else {
          setStatusMessage(
            customEvent.detail.reason ?? `Take ${customEvent.detail.dir} needs a conflict selected`,
          );
        }
      }
    };
    window.addEventListener("autocompare:merge-take-result", handleMergeTakeResult);
    return () => window.removeEventListener("autocompare:merge-take-result", handleMergeTakeResult);
  }, []);

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
          setManagerOpen(true);
          setStatusMessage("Session Manager opened");
          break;
        case "file.open-workspace":
          setManagerOpen(true);
          setStatusMessage(
            "Session Manager opened (open-workspace duplicates session manager; workspace mode pending backend)",
          );
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
        case "session.copy-left":
          handleViewCommandRef.current("take-left");
          break;
        case "session.take-right":
        case "session.copy-right":
          handleViewCommandRef.current("take-right");
          break;
        case "session.take-both":
          handleViewCommandRef.current("take-both");
          break;
        case "session.copy":
        case "edit.copy-hunk":
          handleViewCommandRef.current("copy");
          break;
        case "session.swap":
        case "view.swap":
          handleSwapSides();
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
                const copies = readStoredCopies(left, right);
                setRulerRows(applyStoredCopiesToDiffRows(expanded, copies));
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
          const copies = readStoredCopies(left, right);
          setRulerRows(applyStoredCopiesToDiffRows(all, copies));
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

  // Wave-125 / FIXSHELL GAP-04: intercept localStorage setItem for COPY_STORE_PREFIX so
  // App instantly knows when TextView stages a hunk copy or re-saves.
  useEffect(() => {
    if (typeof window === "undefined" || !window.localStorage) return;
    const origSetItem = window.localStorage.setItem;
    window.localStorage.setItem = function (key: string, val: string) {
      origSetItem.call(window.localStorage, key, val);
      if (typeof key === "string" && key.startsWith(COPY_STORE_PREFIX)) {
        window.dispatchEvent(new CustomEvent("autocompare:staged-copy", { detail: { key } }));
      }
    };
    return () => {
      window.localStorage.setItem = origSetItem;
    };
  }, []);

  // Wave-125 / FIXSHELL GAP-04: refresh ruler + hunk nav on staged copy or save events
  useEffect(() => {
    const refreshRulerCopies = () => {
      if (viewKind !== "text") return;
      const left = activePaths.left;
      const right = activePaths.right;
      const copies = readStoredCopies(left, right);
      setRulerRows((prev) => applyStoredCopiesToDiffRows(prev, copies));
    };

    // FIXRACE: save-to-disk never bumps compareTick. The save receipt
    // (`textview/save-note`) lives in TextView local state under the
    // `key={activeTabId:compareTick}` ViewRouter, so a tick bump here
    // (or on any save-click below) remounts the view ~60ms after Save
    // and wipes the receipt before the dark-tail assertion. Save changes
    // no compare inputs, so only the ruler journal refreshes (staged
    // copies cleared on save); no re-compare, no remount.
    const handleSaveEvent = () => {
      refreshRulerCopies();
    };

    window.addEventListener("storage", refreshRulerCopies);
    window.addEventListener("autocompare:staged-copy", refreshRulerCopies);
    window.addEventListener("autocompare:text-copy", () => {
      setTimeout(refreshRulerCopies, 30);
    });
    window.addEventListener("autocompare:text-copy-line", () => {
      setTimeout(refreshRulerCopies, 30);
    });
    window.addEventListener("autocompare:text-save", handleSaveEvent);
    window.addEventListener("autocompare:session-saved", handleSaveEvent);

    const handleClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;
      if (target.closest('[data-testid^="hunk-copy-"]') || target.closest('[data-testid*="copy-line"]')) {
        setTimeout(refreshRulerCopies, 30);
      } else if (target.closest('[data-testid*="save"]')) {
        setTimeout(handleSaveEvent, 60);
      }
    };
    window.addEventListener("click", handleClick, true);

    return () => {
      window.removeEventListener("storage", refreshRulerCopies);
      window.removeEventListener("autocompare:staged-copy", refreshRulerCopies);
      window.removeEventListener("autocompare:text-save", handleSaveEvent);
      window.removeEventListener("autocompare:session-saved", handleSaveEvent);
      window.removeEventListener("click", handleClick, true);
    };
  }, [viewKind, activePaths.left, activePaths.right]);

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
          case "folder": {
            // C1c: keep the real session handle for Toolbar Stop.
            const fc = await folderCompare(left, right);
            setFolderSessionId(fc.session);
            break;
          }
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
          case "merge": {
            // FIXM GAP-08: merge triple compare (BC 3-way). The base
            // rides `runPaths` (PathBar submit / merge-pane drops keep it
            // staged; the MergeView re-plans from the same triple), so
            // this run validates the triple through the real
            // `merge_text_plan` IPC and snapshots the tick like every
            // other kind. Single-file drops never reach here (guarded
            // above by `!left || !right`).
            const base = runPaths.base ?? left;
            await mergeTextPlanForTriple({ base, left, right });
            break;
          }
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
        // FIXSHELL GAP-07: empty workspace single-file drop opens a new session (BC parity)
        const folderish = isFolderishPath(path);
        const targetKind = folderish ? "folder" : "text";
        const left = side === "left" ? path : "";
        const right = side === "right" ? path : "";
        const switchNote =
          targetKind === "folder"
            ? "Folder dropped — opened folder compare (BC DIRVIEWNEW)"
            : "File dropped — opened text compare (BC FILEVIEWNEW)";
        void (async () => {
          const newId = await handleOpenSession(targetKind, left, right);
          if (newId) {
            setStatusMessage(`${switchNote}, drop the other side to compare`);
          }
        })();
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
      // FIXM N1: merge triple never side-splits here. A single outer
      // drop sets only that side; the base ancestor rides through
      // untouched (the MergeView center zone owns base drops via its
      // `data-mergedrop="base"` zones with stopPropagation; shell drops
      // only carry a left/right half, so base can never arrive here and
      // must never be cleared here).
      const runId = activeSession.id;
      const next: TabPaths = {
        kind: activeSession.kind,
        left: side === "left" ? path : activePaths.left,
        right: side === "right" ? path : activePaths.right,
        ...(activeSession.kind === "merge" && activePaths.base !== undefined
          ? { base: activePaths.base }
          : {}),
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
      const first = paths[0];
      const second = paths[1];
      if (first === undefined || second === undefined) return;
      // FIXM N1: dual-drop into a merge session never auto-switches
      // kind (two paths cannot name a triple side-split). Item 0 ->
      // left, Item 1 -> right, base ancestor preserved; then the same
      // merge-plan compare as the toolbar Compare.
      const mergeSession = activeSession;
      if (mergeSession !== null && mergeSession.kind === "merge" && !isFolderishPath(first) && !isFolderishPath(second)) {
        const mergeRunId = mergeSession.id;
        const live = tabPathsRef.current[mergeRunId] ?? activePaths;
        const mergeNext: TabPaths = {
          kind: "merge",
          left: first,
          right: second,
          ...(live.base !== undefined ? { base: live.base } : {}),
        };
        tabPathsRef.current = { ...tabPathsRef.current, [mergeRunId]: mergeNext };
        setTabPaths(tabPathsRef.current);
        void runComparePaths("merge", mergeNext.left, mergeNext.right, mergeRunId, mergeSession.name, mergeNext);
        return;
      }
      if (!activeSession) {
        // FIXSHELL GAP-07: empty workspace dual-drop opens session and auto-compares
        const anyFolderish = isFolderishPath(first) || isFolderishPath(second);
        const targetKind = anyFolderish ? "folder" : "text";
        void (async () => {
          const newId = await handleOpenSession(targetKind, first, second);
          if (newId && canCompareKind(targetKind)) {
            void runComparePaths(targetKind, first, second, newId, `${targetKind} session`, {
              kind: targetKind,
              left: first,
              right: second,
            });
          }
        })();
        return;
      }
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

  // FIXM GAP-08: merge-pane drop handoff (MergeView 3-way zones call
  // these through ViewRouter). `onDropPaths` stages the full triple in
  // `tabPaths` (the view re-plans automatically once its path props
  // change — no new IPC); `onRequestCompare` runs the same merge-plan
  // compare as the toolbar Compare right after. Non-merge sessions
  // never reach here (MergeView only mounts for merge kind).
  const handleMergeDropTriple = useCallback(
    (next: { readonly leftPath: string; readonly basePath: string; readonly rightPath: string }) => {
      if (!activeSession) {
        setStatusMessage("Drop needs an open session");
        return;
      }
      if (activeSession.kind !== "merge") return;
      const runId = activeSession.id;
      const updated: TabPaths = {
        kind: "merge",
        left: next.leftPath,
        right: next.rightPath,
        base: next.basePath,
      };
      tabPathsRef.current = { ...tabPathsRef.current, [runId]: updated };
      setTabPaths(tabPathsRef.current);
      setStatusMessage("Merge triple staged — auto-compare re-plans from the new triple");
    },
    [activeSession],
  );

  const handleMergeRequestCompare = useCallback(
    (next: { readonly leftPath: string; readonly basePath: string; readonly rightPath: string }) => {
      if (!activeSession || activeSession.kind !== "merge") return;
      const runId = activeSession.id;
      const runPaths: TabPaths = {
        kind: "merge",
        left: next.leftPath,
        right: next.rightPath,
        base: next.basePath,
      };
      void runComparePaths("merge", next.leftPath, next.rightPath, runId, activeSession.name, runPaths);
    },
    [activeSession, runComparePaths],
  );

  // Wave-211: native Finder/Explorer drops subscribe at the WINDOW level
  // (`getCurrentWindow().onDragDropEvent`; v2 has no `onFileDropEvent`).
  // Window events are a superset of the webview surface, so this replaces
  // `useTauriPaneDrop` as the single owner — one drop, one auto-compare.
  // No-op under `VITE_MOCK_IPC=1` / plain browser (HTML5 owns drops there).
  useNativeFileDrop(true, handlePaneDrop, handlePaneDropPaths);

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
      // FIXM GAP-08: merge is comparable (triple plan through
      // `merge_text_plan`; the MergeView re-plans from the same triple).
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

  // Wave-G2: mirror hunk starts for the menu Take fallback above.
  useEffect(() => {
    hunkStartsRef.current = hunkStarts;
  }, [hunkStarts]);

  const handleNextHunk = useCallback(() => {
    if (viewKind !== "text" || hunkStarts.length === 0) return;
    const current = selectedIndex ?? rulerActive;
    let currentHunkIndex = -1;
    for (let i = hunkStarts.length - 1; i >= 0; i -= 1) {
      if (hunkStarts[i]! <= current) {
        currentHunkIndex = i;
        break;
      }
    }
    const next =
      currentHunkIndex < 0
        ? hunkStarts[0]
        : hunkStarts[(currentHunkIndex + 1) % hunkStarts.length];
    if (next !== undefined) {
      handleRulerJump(next);
      setSelectedIndex(next);
      setRulerActive(next);
      // Wave-161: follow-effect for TextView (scroll/focus sync) via the
      // shared bus — TextView's window listener consumes it directly and
      // never re-dispatches, so this cannot loop.
      dispatchViewCommand("next", "text");
    }
  }, [viewKind, hunkStarts, selectedIndex, rulerActive, handleRulerJump]);

  const handlePrevHunk = useCallback(() => {
    if (viewKind !== "text" || hunkStarts.length === 0) return;
    const current = selectedIndex ?? rulerActive;
    let currentHunkIndex = -1;
    for (let i = hunkStarts.length - 1; i >= 0; i -= 1) {
      if (hunkStarts[i]! <= current) {
        currentHunkIndex = i;
        break;
      }
    }
    const prev =
      currentHunkIndex <= 0
        ? hunkStarts[hunkStarts.length - 1]
        : hunkStarts[currentHunkIndex - 1];
    if (prev !== undefined) {
      handleRulerJump(prev);
      setSelectedIndex(prev);
      setRulerActive(prev);
      // Wave-161: follow-effect for TextView (scroll/focus sync) via the
      // shared bus — consumed directly, never re-dispatched, no loop.
      dispatchViewCommand("prev", "text");
    }
  }, [viewKind, hunkStarts, selectedIndex, rulerActive, handleRulerJump]);

  const hunkNavEnabled = viewKind === "text" && hunkStarts.length > 0;
  // FIXM GAP-13: merge conflict nav is bus-owned (MergeView wraps +
  // no-ops honestly when clean), so prev/next stay enabled in merge
  // with conflict labels; other non-text kinds keep the parked label.
  const mergeNavActive = viewKind === "merge";

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
  // FIXSHELL GAP-05: support take-right on changed lines
  const [takeSidePref, setTakeSidePref] = useState<TextSide>("left");

  // Track cell click to prefer clicked side on changed lines
  useEffect(() => {
    const handleCellClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;
      const cell = target.closest(".tv-cell");
      if (cell) {
        if (cell.classList.contains("order-last") || cell.getAttribute("data-side") === "right") {
          setTakeSidePref("right");
        } else {
          setTakeSidePref("left");
        }
      }
    };
    window.addEventListener("click", handleCellClick, true);
    return () => window.removeEventListener("click", handleCellClick, true);
  }, []);

  const copyTakeSide: TextSide | null =
    viewKind === "text" && selectedRow !== null
      ? selectedRow.kind === "changed"
        ? takeSidePref
        : selectedRow.left !== null
          ? "left"
          : "right"
      : null;
  const copyInHunk = copyTakeSide !== null && selectedRow !== null && selectedRow.kind !== "equal";
  const mergeLive = sessionKind === "merge";

  const handleToolbarCopy = useCallback(
    (forcedSide?: TextSide) => {
      const dir = forcedSide ?? copyTakeSide ?? "left";
      handleMenuTake(dir);
    },
    [handleMenuTake, copyTakeSide],
  );

  // FIXSHELL GAP-09 + FIXM N6-app-part: difference navigation (n/p)
  // and section copy (l/r) from body/toolbar/gutter/ruler focus (BC
  // parity). Text keeps the live hunk handlers; merge routes through
  // the `autocompare:view-command` bus (`viewKind: "merge"`, owned by
  // the MergeView listener) and l/r ride `handleMenuTake` (which already
  // routes merge takes to `autocompare:merge-take`). Folder/hex/table/
  // sync/media keep the dead guard below — no view owns n/p/l/r there
  // yet (toolbar parks them honestly instead of pretending).
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.defaultPrevented) return;
      const target = e.target as HTMLElement | null;
      const activeEl =
        typeof document !== "undefined" ? (document.activeElement as HTMLElement | null) : null;
      const isEditable = (el: HTMLElement | null): boolean => {
        if (!el) return false;
        const tag = el.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
        if (el.isContentEditable || el.getAttribute("contenteditable") === "true") return true;
        if (
          el.closest?.("input, textarea, select, [contenteditable='true'], [contenteditable='']")
        ) {
          return true;
        }
        return false;
      };
      if (isEditable(target) || isEditable(activeEl)) {
        return;
      }
      const isCmdAlt = (e.metaKey || e.ctrlKey) && e.altKey && !e.shiftKey;
      const isPlain = !e.metaKey && !e.ctrlKey && !e.altKey;
      if (!isPlain && !isCmdAlt) return;

      const keyKind = viewKindRef.current ?? viewKind;
      if (keyKind !== "text" && keyKind !== "merge") return;

      const key = e.key.toLowerCase();
      const isNext = (isPlain && key === "n") || (isCmdAlt && (e.key === "ArrowDown" || e.key === "Down"));
      const isPrev = (isPlain && key === "p") || (isCmdAlt && (e.key === "ArrowUp" || e.key === "Up"));

      if (isNext) {
        e.preventDefault();
        // FIXM: no `handleViewCommand` ref here (declared below; naming
        // it in the deps array would hit TDZ at render). Merge nav
        // dispatches the same bus event inline.
        if (keyKind === "merge") {
          dispatchViewCommand("next", "merge");
          setStatusMessage("Next conflict");
        } else handleNextHunk();
      } else if (isPrev) {
        e.preventDefault();
        if (keyKind === "merge") {
          dispatchViewCommand("prev", "merge");
          setStatusMessage("Previous conflict");
        } else handlePrevHunk();
      } else if (isPlain && key === "l") {
        e.preventDefault();
        setTakeSidePref("left");
        handleMenuTake("left");
      } else if (isPlain && key === "r") {
        e.preventDefault();
        setTakeSidePref("right");
        handleMenuTake("right");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [viewKind, handleNextHunk, handlePrevHunk, handleMenuTake]);

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

  // Wave-161: ViewCommandBus (`components/chrome/ViewCommandBus`) — ONE
  // routing point for prev/next/copy/swap per active viewKind (BC
  // unified-toolbar parity). Text keeps the live hunk handlers above;
  // FIXM GAP-13: merge dispatches the bus with `viewKind: "merge"`
  // (MergeView owns prev/next/copy incl. `dir`); every other kind gets
  // an honest parked status note (per-view wiring lands in L3), never a
  // silent no-op. The top Toolbar calls this bus directly; TextToolbar
  // rides the SAME bus via the `onTextBusCommand` prop on ViewRouter
  // (forwarded to TextView `onBusCommand`) — no duplicate handlers.
  // TextView's internal window listener stays a pure follow-effect
  // (scroll/focus sync), never a second invocation.
  const handleViewCommand = useCallback(
    (cmd: ViewCommand): void => {
      const kind = viewKindRef.current ?? viewKind;
      switch (cmd) {
        case "prev":
        case "next": {
          if (kind === "merge") {
            dispatchViewCommand(cmd, "merge");
            setStatusMessage(cmd === "prev" ? "Previous conflict" : "Next conflict");
            return;
          }
          if (kind !== "text") {
            setStatusMessage(
              `${cmd === "prev" ? "Previous" : "Next"} hunk needs view wiring in L3`,
            );
            return;
          }
          if (hunkStarts.length === 0) {
            setStatusMessage("No hunks to navigate");
            return;
          }
          if (cmd === "prev") handlePrevHunk();
          else handleNextHunk();
          return;
        }
        case "copy": {
          // FIXM GAP-13: merge copy resolves the active conflict
          // (default take-left; take-right rides `copy-right` below).
          // The bus branch is live for bus/programmatic callers; the top
          // toolbar Copy stays disabled in merge (conflict selection is
          // owned by MergeView — per-conflict Take buttons + menu Take
          // are the enabled paths there), so this never fires silently.
          if (kind === "merge") {
            dispatchViewCommand(cmd, "merge", { dir: "left" });
            setStatusMessage("Copy take-left requested (merge)");
            return;
          }
          if (kind !== "text") {
            setStatusMessage(`Copy needs view wiring in L3 for kind "${kind ?? "none"}"`);
            return;
          }
          handleToolbarCopy();
          return;
        }
        case "copy-right": {
          if (kind === "merge") {
            dispatchViewCommand(cmd, "merge", { dir: "right" });
            setStatusMessage("Copy take-right requested (merge)");
            return;
          }
          if (kind !== "text") {
            setStatusMessage(`Copy needs view wiring in L3 for kind "${kind ?? "none"}"`);
            return;
          }
          handleToolbarCopy("right");
          return;
        }
        case "swap": {
          handleSwapSides();
          return;
        }
        case "merge": {
          handleMergeJump();
          return;
        }
        case "take-left": {
          if (kind === "merge") {
            dispatchViewCommand("copy", "merge", { dir: "left" });
            setStatusMessage("Copy take-left requested (merge)");
            return;
          }
          if (kind !== "text") {
            setStatusMessage(`Copy needs view wiring in L3 for kind "${kind ?? "none"}"`);
            return;
          }
          handleMenuTake("left");
          return;
        }
        case "take-right": {
          if (kind === "merge") {
            dispatchViewCommand("copy", "merge", { dir: "right" });
            setStatusMessage("Copy take-right requested (merge)");
            return;
          }
          if (kind !== "text") {
            setStatusMessage(`Copy needs view wiring in L3 for kind "${kind ?? "none"}"`);
            return;
          }
          handleMenuTake("right");
          return;
        }
        case "take-both": {
          if (kind === "merge") {
            dispatchViewCommand("copy", "merge", { dir: "both" });
            setStatusMessage("Copy take-both requested (merge)");
            return;
          }
          setStatusMessage("Take both is only available in Merge view");
          return;
        }
      }
    },
    [viewKind, hunkStarts, handlePrevHunk, handleNextHunk, handleToolbarCopy, handleMenuTake, handleSwapSides, handleMergeJump],
  );
  handleViewCommandRef.current = handleViewCommand;

  // Wave-161: unified filter dispatch (BC View: Show All/Differences/Same/
  // Context) through the ViewCommandBus. Every view listens for
  // `autocompare:view-filter` already; the dispatch + status note are
  // honest either way and the view-level controls keep working
  // untouched (their testids stay pinned — single dispatch point, no
  // scattered duplicate dispatches).
  const handleViewFilter = useCallback(
    (filter: ViewFilterKind): void => {
      if (activeSession === null) {
        setStatusMessage("Filters need an open session");
        return;
      }
      const kind = viewKindRef.current ?? viewKind;
      dispatchViewFilter(filter, kind);
      setStatusMessage(`View filter: ${filter}`);
    },
    [activeSession, viewKind],
  );

  // Wave-161: Minor is LIVE in text kind — it toggles BOTH text IPC
  // ignore options (`ignoreWhitespace` + `ignoreCase`) through the
  // existing `text_compare` path (TextView `autocompare:view-minor`
  // listener rides `recompareWithIgnores`, no engine change).
  // Wave-169: Minor extends to table kind (dispatchViewMinor with kind
  // table through existing view bus + table IPC ignore flags; text
  // behavior unchanged). Other kinds keep the honest parked state.
  // Wave-167: Rules/Format ride the same bus pattern (see
  // `handleRulesToggle`/`handleFormatToggle` below).
  const [minorOn, setMinorOn] = useState(false);
  // Mirror TextView's session-switch ignore reset (its load effect
  // clears ws/case on path/session change, and a remount on
  // `compareTick` drops them too): the toggle resets with the tab, the
  // paths, and the compare generation so the button never claims "on"
  // while the view runs exact.
  useEffect(() => {
    setMinorOn(false);
  }, [activeTabId, activePaths.left, activePaths.right, compareTick]);
  const minorLive =
    (viewKind === "text" || viewKind === "table") && activeSession !== null;
  const handleMinorToggle = useCallback(() => {
    if (!minorLive) {
      setStatusMessage("Minor needs text compare");
      return;
    }
    const next = !minorOn;
    setMinorOn(next);
    if (viewKind === "table") {
      // Wave-180: bus ONLY — TableView applies it to ignoreWs/ignoreCase
      // and its compare effect re-issues `table_compare` (single source,
      // no parallel direct IPC).
      dispatchViewMinor(next, "table");
    } else {
      dispatchViewMinor(next, "text");
    }
    setStatusMessage(
      next ? "Minor: ignoring whitespace and case" : "Minor off: exact diff",
    );
  }, [minorLive, minorOn, viewKind]);
  // Wave-167: Rules/Format are LIVE top-bar actions (BC Session >
  // Settings parity). Rules opens the in-view Rules panel in folder
  // kind (`autocompare:view-rules`, owned by FolderView); Format opens
  // the in-view Format menu in text kind (`autocompare:view-format`,
  // owned by TextView). Other kinds keep the honest parked reason —
  // the panel/menu simply does not exist there, so the button parks
  // instead of pretending.
  const rulesLive = viewKind === "folder" && activeSession !== null;
  const formatLive = viewKind === "text" && activeSession !== null;
  const handleRulesToggle = useCallback(() => {
    if (!rulesLive) {
      setStatusMessage("Rules needs folder compare");
      return;
    }
    dispatchViewRules();
    setStatusMessage("Rules panel opened");
  }, [rulesLive]);
  const handleFormatToggle = useCallback(() => {
    if (!formatLive) {
      setStatusMessage("Format needs text compare");
      return;
    }
    dispatchViewFormat();
    setStatusMessage("Format options opened");
  }, [formatLive]);

  const filterEnabled = activeSession !== null;
  const filterReason = filterEnabled ? null : "needs an open session";

  const toolbarActions: readonly ToolbarAction[] = [
    {
      id: "home",
      icon: Home,
      label: "Home",
      enabled: true,
      onClick: () => handleSelectTab(WELCOME_TAB.id),
    },
    { id: "open", icon: FolderOpen, label: "Open session", enabled: true, onClick: () => setManagerOpen(true) },
    {
      // Wave-161: lifted BC View filters — the single dispatch point for
      // All/Diffs/Same/Context (every view listens on the bus; no
      // scattered duplicate dispatches). View-level controls keep their
      // own testids and keep working untouched.
      id: "filter-all",
      icon: List,
      label: "Show all",
      enabled: filterEnabled,
      disabledReason: filterReason,
      onClick: () => handleViewFilter("all"),
    },
    {
      id: "filter-diffs",
      icon: Filter,
      label: "Show differences",
      enabled: filterEnabled,
      disabledReason: filterReason,
      onClick: () => handleViewFilter("diffs"),
    },
    {
      id: "filter-same",
      icon: Equal,
      label: "Show same",
      enabled: filterEnabled,
      disabledReason: filterReason,
      onClick: () => handleViewFilter("same"),
    },
    {
      id: "filter-context",
      icon: AlignJustify,
      label: "Context",
      enabled: filterEnabled,
      disabledReason: filterReason,
      onClick: () => handleViewFilter("context"),
    },
    {
      // Wave-161 / Wave-169: Minor is LIVE in text and table kinds
      // (toggles ignore options through the existing compare path, no engine
      // change); every other kind keeps the honest parked reason.
      id: "minor",
      icon: Minus,
      label: minorOn ? "Minor (on)" : "Minor",
      enabled: minorLive,
      disabledReason: minorLive ? null : "needs text compare",
      onClick: handleMinorToggle,
    },
    {
      // Wave-167: LIVE in folder kind (opens the in-view Rules panel
      // through the view bus); honestly parked everywhere else.
      id: "rules",
      icon: Code,
      label: rulesLive ? "Rules" : "Rules (needs folder compare)",
      enabled: rulesLive,
      disabledReason: rulesLive ? null : "needs folder compare",
      onClick: handleRulesToggle,
    },
    {
      // Wave-167: LIVE in text kind (opens the in-view Format menu
      // through the view bus); honestly parked everywhere else.
      id: "format",
      icon: Type,
      label: formatLive ? "Format" : "Format (needs text compare)",
      enabled: formatLive,
      disabledReason: formatLive ? null : "needs text compare",
      onClick: handleFormatToggle,
    },
    {
      // Wave-125: live in text kind with a hunk row selected (same
      // path as the gutter arrows); parked with an honest reason
      // otherwise. FIXM GAP-13: merge keeps Copy disabled honestly —
      // conflict selection lives in MergeView (per-conflict Take
      // buttons + menu Take Left/Right/Both + the bus `copy` branch
      // are the live merge paths), never a silent no-op.
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
      // Wave-L2: bus-routed (text rides the staged text-copy path;
      // merge resolves the active conflict; other kinds park with the
      // L3-wiring reason).
      onClick: () => handleViewCommand("copy"),
    },
    {
      // Wave-L2: bus-routed, same per-kind honesty as prev-hunk.
      id: "next-hunk",
      icon: ChevronDown,
      label:
        hunkNavEnabled || viewKind === "text"
          ? hunkNavEnabled
            ? "Next hunk"
            : "Next hunk (no hunks)"
          : mergeNavActive
            ? "Next conflict"
            : "Next hunk (needs view wiring in L3)",
      enabled: hunkNavEnabled || mergeNavActive,
      disabledReason: hunkNavEnabled || mergeNavActive ? null : viewKind === "text" ? "no hunks" : "needs view wiring in L3",
      onClick: () => handleViewCommand("next"),
    },
    {
      // Wave-L2: bus-routed (text keeps the live hunk handlers; FIXM
      // GAP-13: merge dispatches the conflict bus, staying enabled with
      // conflict labels; other kinds park with the L3-wiring reason).
      id: "prev-hunk",
      icon: ChevronUp,
      label:
        hunkNavEnabled || viewKind === "text"
          ? hunkNavEnabled
            ? "Previous hunk"
            : "Previous hunk (no hunks)"
          : mergeNavActive
            ? "Previous conflict"
            : "Previous hunk (needs view wiring in L3)",
      enabled: hunkNavEnabled || mergeNavActive,
      disabledReason: hunkNavEnabled || mergeNavActive ? null : viewKind === "text" ? "no hunks" : "needs view wiring in L3",
      onClick: () => handleViewCommand("prev"),
    },
    {
      id: "swap",
      icon: ArrowLeftRight,
      label: "Swap sides",
      enabled: activeSession !== null,
      disabledReason: activeSession !== null ? null : "needs an open session",
      onClick: () => handleViewCommand("swap"),
    },
    {
      id: "compare",
      icon: Columns2,
      label: compareBlockedReason ? `Compare (${compareBlockedReason})` : "Compare",
      enabled: compareBlockedReason === null,
      disabledReason: compareBlockedReason,
      onClick: handleCompare,
    },
    {
      id: "copy-right",
      icon: Copy,
      label: copyInHunk
        ? "Copy (take right)"
        : viewKind === "text" && selectedRow !== null
          ? "Copy (selected row is not a difference)"
          : "Copy (select a row first)",
      enabled: copyInHunk && selectedRow?.right !== null,
      disabledReason: copyInHunk
        ? null
        : viewKind === "text" && selectedRow !== null
          ? "selected row is not a difference"
          : "select a row first",
      onClick: () => handleViewCommand("copy-right"),
    },
    {
      id: "sync",
      icon: RefreshCw,
      label: syncEnabled ? "Sync" : "Sync (needs folder session)",
      enabled: syncEnabled,
      disabledReason: syncEnabled ? null : "needs folder session",
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
      onClick: () => handleViewCommand("merge"),
    },
  ];

  const inlineError = actionError ?? sessionError;
  // Wave-10: demo banner shows only on the auto-opened demo tab.
  const isDemo = demoId !== null && activeSession?.id === demoId;

  // Wave-232: dev-only ui gallery at /?gallery=1 (no views/chrome, no IPC).
  if (typeof window !== "undefined" && new URLSearchParams(window.location.search).get("gallery") === "1") {
    return <Gallery />;
  }
  return (
    <DiffStatsContext.Provider value={diffStats}>
      <ChromeShell
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={handleSelectTab}
        onCloseTab={handleCloseTab}
        onNewTab={handleNewTab}
        toolbar={{ actions: toolbarActions }}
        // C1c: Stop cancels ONLY the live folder session — other kinds
        // (or no run yet) pass null so Stop keeps the honest-note path.
        folderSession={sessionKind === "folder" ? folderSessionId : null}
        leftPath={activePaths.left}
        rightPath={activePaths.right}
        onSubmitPath={handlePathSubmit}
        basePath={activePaths.base ?? ""}
        showBase={sessionKind === "merge"}
        // Wave-140: PathBar/strip dual-drop -> single auto-compare
        // (BC Item0 -> left, Item1 -> right; side arg ignored for dual).
        onDropPaths={(paths) => handlePaneDropPaths(paths, "left")}
        status={{
          // Wave-D3: session count mirrored in the footer line (BC status
          // affiliation); in-pane `session-count` notes stay as subtle
          // mirrors with their pinned testids.
          left: activeSession ? `session: ${activeSession.name}` : "session: none",
          right: activeSession
            ? `${activeSession.kind} \u2022 ${activeSession.pageCount} tab(s) \u2022 sessions: ${visibleSessions.length}`
            : `0 files \u2022 sessions: ${visibleSessions.length}`,
        }}
        statusMessage={statusMessage}
        diffRows={rulerRows}
        onJump={handleRulerJump}
        activeIndex={rulerActive}
        encoding={activeSession ? "UTF-8" : undefined}
        lineCol={
          selectedRow
            ? `Ln ${selectedRow.index + 1}, Col 1`
            : activeSession
              ? "Ln 1, Col 1"
              : undefined
        }
        zoom={activeSession ? "100%" : undefined}
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
        <PaneDropZone
          enabled={true}
          onDropPath={handlePaneDrop}
          onDropPaths={handlePaneDropPaths}
          className="flex min-h-0 flex-1 flex-col"
        >
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
              {/* H1: Home canvas par stray monospace `sessions:N` nahi (BC
                  jaisa clean canvas). Count status bar me pehle se mirror
                  hai (`status.right` sessions:N); testid byte-identical
                  rakhkar sr-only — specs `session-count` text se padh
                  sakte hain, canvas par dikhta nahi. */}
              <p className="sr-only" data-testid="session-count">
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
              <p className="ac-mono self-stretch text-right text-[10px] leading-none text-[var(--ac-fg-muted)] opacity-60" data-testid="session-count">
                sessions: {visibleSessions.length}
              </p>
            </div>
          ) : viewKind ? (
            <>
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
              {/* FIXM GAP-08: merge host wiring — pane-drop triple staging
                  + auto-compare routing. `onBusCommand` (merge) is
                  deliberately NOT passed: the MergeView internal
                  `autocompare:view-command` listener owns the bus, and a
                  re-dispatching prop would loop back into itself.
                  `onTextBusCommand` below is text-only (ViewRouter
                  forwards it to TextView alone), so merge is unaffected. */}
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
                onDropPaths={handleMergeDropTriple}
                onRequestCompare={handleMergeRequestCompare}
                // Wave-161: TextToolbar prev/next ride the SAME bus as
                // the top Toolbar (`handleViewCommand`) — no duplicate
                // handlers. Merge keeps its internal listener (see the
                // comment the L2 block below preserves).
                onTextBusCommand={(cmd: "prev" | "next") => handleViewCommand(cmd)}
              />
              {/* Wave-D3: session counter lives BELOW the view as a subtle
                  footer-affiliated note (was a top stacked row overlapping the
                  empty-state/ruler notes). testid + text byte-identical. */}
              <p className="ac-mono mt-auto pointer-events-none select-none px-4 pb-1.5 text-right text-[10px] leading-none text-[var(--ac-fg-muted)] opacity-60" data-testid="session-count">
                sessions: {visibleSessions.length}
              </p>
            </>
          ) : (
            <div className="mx-auto flex max-w-3xl flex-col items-center gap-6 px-8 py-16 text-center" data-testid="view-router">
              <h1 className="ac-display font-semibold text-[var(--ac-fg)]">AutoCompare</h1>
              <p className="ac-body text-[var(--ac-fg-muted)]">
                No data view for kind &ldquo;{activeSession.kind}&rdquo; yet. Manage paths in the Session Manager.
              </p>
              <p className="ac-mono self-stretch text-right text-[10px] leading-none text-[var(--ac-fg-muted)] opacity-60" data-testid="session-count">
                sessions: {visibleSessions.length}
              </p>
            </div>
          )}
        </PaneDropZone>
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
