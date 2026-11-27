import { useCallback, useEffect, useRef, useState } from "react";
import type React from "react";
import { sessionSave } from "../../ipc/session";
import type { SessionKind, V2Tab } from "../types";
import { getComparator, inferComparatorForPaths } from "../plugins/registry";
import { inferDropKind } from "../utils/dropKind";
import {
  isRestoreLastSessionEnabled,
  loadLastSessionSnapshot,
  saveLastSessionSnapshot,
} from "../utils/sessionPrefs";

export { inferDropKind };

export const HOME_TAB: V2Tab = {
  id: "home",
  kind: "home",
  title: "Home",
  leftPath: "",
  rightPath: "",
};

export interface UseTabsOptions {
  readonly onResetCommands: () => void;
}

export function useTabs({ onResetCommands }: UseTabsOptions) {
  const [tabs, setTabs] = useState<V2Tab[]>(() => {
    if (isRestoreLastSessionEnabled()) {
      const snap = loadLastSessionSnapshot();
      if (snap) {
        return [
          HOME_TAB,
          {
            id: `restored-${snap.kind}`,
            kind: snap.kind,
            title: snap.title,
            leftPath: snap.leftPath,
            rightPath: snap.rightPath,
            dirty: false,
          },
        ];
      }
    }
    return [HOME_TAB];
  });
  const [activeId, setActiveId] = useState<string>(() => {
    if (isRestoreLastSessionEnabled()) {
      const snap = loadLastSessionSnapshot();
      if (snap) return `restored-${snap.kind}`;
    }
    return "home";
  });
  const [saveToast, setSaveToast] = useState<string | null>(null);
  const toastTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const showToast = useCallback((message: string, durationMs = 2200): void => {
    setSaveToast(message);
    if (toastTimerRef.current !== null) clearTimeout(toastTimerRef.current);
    toastTimerRef.current = setTimeout(() => {
      setSaveToast(null);
      toastTimerRef.current = null;
    }, durationMs);
  }, []);

  const activeTab = tabs.find((t) => t.id === activeId) ?? HOME_TAB;
  const activeTabRef = useRef<V2Tab>(activeTab);
  activeTabRef.current = activeTab;

  useEffect(() => {
    if (activeTab.kind !== "home") {
      saveLastSessionSnapshot({
        kind: activeTab.kind,
        leftPath: activeTab.leftPath,
        rightPath: activeTab.rightPath,
        title: activeTab.title,
      });
    }
  }, [activeTab.kind, activeTab.leftPath, activeTab.rightPath, activeTab.title]);

  const openSession = useCallback(
    (kind: SessionKind, left = "", right = "", customTitle?: string): void => {
      if (kind === "home") {
        setActiveId("home");
        return;
      }
      const id = `${kind}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
      const kindLabel =
        getComparator(kind)?.title ??
        `${kind[0].toUpperCase()}${kind.slice(1)} Compare`;
      const baseTitle =
        customTitle ??
        (left && right
          ? `${left.split("/").pop() ?? left} ↔ ${right.split("/").pop() ?? right}`
          : kindLabel);
      const next: V2Tab = {
        id,
        kind,
        title: baseTitle,
        leftPath: left,
        rightPath: right,
        dirty: false,
      };
      setTabs((prev) => [...prev, next]);
      setActiveId(id);
      onResetCommands();
    },
    [onResetCommands],
  );

  const closeTab = useCallback(
    (id: string, e?: React.MouseEvent): void => {
      e?.stopPropagation();
      if (id === "home") return;
      setTabs((prev) => {
        const idx = prev.findIndex((t) => t.id === id);
        const next = prev.filter((t) => t.id !== id);
        if (activeId === id) {
          const fallback = next[Math.max(0, idx - 1)] ?? HOME_TAB;
          setActiveId(fallback.id);
        }
        return next;
      });
    },
    [activeId],
  );

  const updateActivePaths = useCallback(
    (left: string, right: string): void => {
      setTabs((prev) =>
        prev.map((t) => {
          if (t.id !== activeId) return t;
          const title =
            left && right
              ? `${left.split("/").pop() ?? left} ↔ ${right.split("/").pop() ?? right}`
              : t.title;
          return { ...t, leftPath: left, rightPath: right, title };
        }),
      );
    },
    [activeId],
  );

  const updateActiveDirty = useCallback(
    (dirty: boolean): void => {
      setTabs((prev) => prev.map((t) => (t.id === activeId ? { ...t, dirty } : t)));
    },
    [activeId],
  );

  const handleSaveWorkspaceSession = useCallback(async (): Promise<void> => {
    const tab = activeTabRef.current;
    if (tab.kind === "home") return;
    try {
      await sessionSave({
        schema_version: 1,
        name: tab.title,
        locked: false,
        active_tab_index: 0,
        tabs: [
          {
            id: tab.id,
            title: tab.title,
            session_type: tab.kind,
            left_path: tab.leftPath || null,
            right_path: tab.rightPath || null,
            base_path: null,
            output_path: null,
            read_only: false,
            locked: false,
            options: {},
          },
        ],
      });
      showToast("Session saved");
    } catch {
      showToast("Could not save session");
    }
  }, [showToast]);

  const handleDroppedPaths = useCallback(
    (paths: readonly string[], side: "left" | "right"): void => {
      const clean = paths.map((p) => p.trim()).filter((p) => p.length > 0);
      if (clean.length === 0) return;

      const cur = activeTabRef.current;
      if (clean.length >= 2) {
        const kind = cur.kind === "home" ? inferComparatorForPaths(clean) : cur.kind;
        if (cur.kind === "home") {
          openSession(kind, clean[0], clean[1]);
        } else {
          updateActivePaths(clean[0], clean[1]);
        }
        showToast(`Loaded 2 paths (${kind})`);
        return;
      }

      const dropped = clean[0];
      if (cur.kind === "home") {
        const kind = inferComparatorForPaths([dropped]);
        openSession(kind, side === "left" ? dropped : "", side === "right" ? dropped : "");
      } else {
        const nextL = side === "left" ? dropped : cur.leftPath;
        const nextR = side === "right" ? dropped : cur.rightPath;
        updateActivePaths(nextL, nextR);
      }
      showToast(`Dropped onto ${side} pane`, 1800);
    },
    [openSession, updateActivePaths, showToast],
  );

  return {
    tabs,
    activeId,
    setActiveId,
    activeTab,
    activeTabRef,
    saveToast,
    showToast,
    openSession,
    closeTab,
    updateActivePaths,
    updateActiveDirty,
    handleSaveWorkspaceSession,
    handleDroppedPaths,
  };
}
