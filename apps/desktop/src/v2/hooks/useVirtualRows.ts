import { useCallback, useEffect, useState } from "react";
import type React from "react";

export interface UseVirtualRowsOptions {
  readonly count: number;
  readonly rowHeight: number;
  readonly overscan?: number;
  readonly scrollRef: React.RefObject<HTMLDivElement | null>;
}

export interface VirtualWindow {
  readonly startIndex: number;
  readonly endIndex: number;
  readonly totalHeight: number;
  readonly offsetTop: number;
  readonly scrollToPosition: (position: number) => void;
}

/**
 * Lightweight fixed-row-height virtualizer for 10k+ line text diffs.
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

  const totalHeight = count * rowHeight;
  const visibleCount = Math.ceil(viewportHeight / rowHeight);
  const rawStart = Math.floor(scrollTop / rowHeight) - overscan;
  const startIndex = Math.max(0, Math.min(count, rawStart));
  const endIndex = Math.min(count, startIndex + visibleCount + overscan * 2);
  const offsetTop = startIndex * rowHeight;

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
