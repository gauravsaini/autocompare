import { useCallback, useEffect, useState } from "react";
import type React from "react";

export interface UseVirtualRowsOptions {
  readonly count: number;
  readonly rowHeight: number;
  readonly overscan?: number;
  readonly scrollRef: React.RefObject<HTMLDivElement | null>;
}

export interface VirtualWindowRange {
  readonly startIndex: number;
  readonly endIndex: number;
  readonly totalHeight: number;
  readonly offsetTop: number;
}

export interface VirtualWindow extends VirtualWindowRange {
  readonly scrollToPosition: (position: number) => void;
}

/** Pure window math (unit-tested in Node without DOM). */
export function computeVirtualWindow(
  count: number,
  rowHeight: number,
  scrollTop: number,
  viewportHeight: number,
  overscan = 16,
): VirtualWindowRange {
  const safeCount = Math.max(0, Math.floor(count));
  const safeRowHeight = Math.max(1, rowHeight);
  const totalHeight = safeCount * safeRowHeight;
  const visibleCount = Math.ceil(Math.max(0, viewportHeight) / safeRowHeight);
  const rawStart = Math.floor(Math.max(0, scrollTop) / safeRowHeight) - overscan;
  const startIndex = Math.max(0, Math.min(safeCount, rawStart));
  const endIndex = Math.min(safeCount, startIndex + visibleCount + overscan * 2);
  const offsetTop = startIndex * safeRowHeight;
  return { startIndex, endIndex, totalHeight, offsetTop };
}

/**
 * Lightweight fixed-row-height virtualizer for 10k+ line diffs, tables, and folder trees.
 * Renders only the visible viewport slice plus `overscan` rows above/below.
 */
export function useVirtualRows({
  count,
  rowHeight,
  overscan = 16,
  scrollRef,
}: UseVirtualRowsOptions): VirtualWindow {
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(640);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const updateSize = (): void => {
      if (el.clientHeight > 0) setViewportHeight(el.clientHeight);
    };
    updateSize();

    let rafId: number | null = null;
    const onScroll = (): void => {
      if (rafId !== null) return;
      rafId = requestAnimationFrame(() => {
        rafId = null;
        setScrollTop(el.scrollTop);
      });
    };

    el.addEventListener("scroll", onScroll, { passive: true });
    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      ro = new ResizeObserver(updateSize);
      ro.observe(el);
    }

    return () => {
      el.removeEventListener("scroll", onScroll);
      if (rafId !== null) cancelAnimationFrame(rafId);
      ro?.disconnect();
    };
  }, [scrollRef]);

  const { startIndex, endIndex, totalHeight, offsetTop } = computeVirtualWindow(
    count,
    rowHeight,
    scrollTop,
    viewportHeight,
    overscan,
  );

  const scrollToPosition = useCallback(
    (position: number): void => {
      const el = scrollRef.current;
      if (!el || count === 0) return;
      const clamped = Math.max(0, Math.min(count - 1, position));
      const targetTop = Math.max(0, clamped * rowHeight - el.clientHeight / 2 + rowHeight / 2);
      el.scrollTo({ top: targetTop, behavior: "smooth" });
      setScrollTop(targetTop);
    },
    [scrollRef, count, rowHeight],
  );

  return { startIndex, endIndex, totalHeight, offsetTop, scrollToPosition };
}
