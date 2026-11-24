/**
 * Wave-7 overlap helper: zero text-overlap assertion for leaf-wave specs.
 *
 * Same boundingBox technique that tools/loop-harness/screens.cjs uses for
 * headless capture proof, exposed as a Playwright assertion so future
 * waves (8+) can pin layout in-repo:
 *
 *   import { assertNoOverlap } from "./helpers/overlap";
 *   await assertNoOverlap(page);
 *
 * Ancestor/descendant nesting is NOT a violation (parents contain
 * children); only non-containing boxes with intersecting area fail.
 */
import { expect, type Page } from "@playwright/test";

export const OVERLAP_SELECTORS = "header, nav, main, [data-testid]";

export interface Box {
  readonly sel: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export function intersectArea(a: Box, b: Box): number {
  const x1 = Math.max(a.x, b.x);
  const y1 = Math.max(a.y, b.y);
  const x2 = Math.min(a.x + a.width, b.x + b.width);
  const y2 = Math.min(a.y + a.height, b.y + b.height);
  return Math.max(0, x2 - x1) * Math.max(0, y2 - y1);
}

export function containsBox(a: Box, b: Box, tol = 2): boolean {
  return (
    a.x - tol <= b.x &&
    a.y - tol <= b.y &&
    a.x + a.width + tol >= b.x + b.width &&
    a.y + a.height + tol >= b.y + b.height
  );
}

export async function snapshotBoxes(page: Page, selectors = OVERLAP_SELECTORS): Promise<Box[]> {
  return page.$$eval(selectors, (els) =>
    els
      .map((el) => {
        const r = el.getBoundingClientRect();
        const sel = el.getAttribute("data-testid") ?? el.tagName.toLowerCase();
        return { sel, x: r.x, y: r.y, width: r.width, height: r.height };
      })
      .filter((b) => b.width > 0 && b.height > 0),
  );
}

/** Fails the test if any two non-nested visible boxes overlap. */
export async function assertNoOverlap(page: Page, selectors = OVERLAP_SELECTORS): Promise<void> {
  const boxes = await snapshotBoxes(page, selectors);
  const violations: string[] = [];
  for (let i = 0; i < boxes.length; i++) {
    for (let j = i + 1; j < boxes.length; j++) {
      const a = boxes[i];
      const b = boxes[j];
      if (containsBox(a, b) || containsBox(b, a)) continue;
      if (intersectArea(a, b) > 1) violations.push(`${a.sel} OVERLAPS ${b.sel}`);
    }
  }
  expect(violations, `text overlap: ${violations.slice(0, 10).join("; ")}`).toEqual([]);
}
