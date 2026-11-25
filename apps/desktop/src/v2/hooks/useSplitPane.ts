import { useCallback, useEffect, useRef, useState } from "react";

export interface SplitPaneHandle {
  readonly containerRef: React.RefObject<HTMLDivElement | null>;
  readonly splitPct: number;
  readonly dragging: boolean;
  readonly startDrag: (e: React.MouseEvent) => void;
  readonly resetSplit: () => void;
}

/**
 * Smooth draggable Left/Right splitter hook for AutoCompare v2 panes.
 * Clamps between 25% and 75%; double-click resets cleanly to 50%.
 */
export function useSplitPane(initialPct = 50): SplitPaneHandle {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [splitPct, setSplitPct] = useState(initialPct);
  const [dragging, setDragging] = useState(false);

  const startDrag = useCallback((e: React.MouseEvent): void => {
    e.preventDefault();
    e.stopPropagation();
    setDragging(true);
  }, []);

  const resetSplit = useCallback((): void => {
    setSplitPct(50);
  }, []);

  useEffect(() => {
    if (!dragging) return;
    const prevCursor = document.body.style.cursor;
    const prevSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMove = (e: MouseEvent): void => {
      const el = containerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0) return;
      const pct = ((e.clientX - rect.left) / rect.width) * 100;
      setSplitPct(Math.max(25, Math.min(75, Math.round(pct * 10) / 10)));
    };

    const onUp = (): void => {
      setDragging(false);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      document.body.style.cursor = prevCursor;
      document.body.style.userSelect = prevSelect;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, [dragging]);

  return { containerRef, splitPct, dragging, startDrag, resetSplit };
}
