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
 *
 * Wave-220 overlay opt-out (explicit, honest, NOT a global weakening):
 * elements inside a `[data-overlay]` subtree float above content BY
 * DESIGN (Beyond Compare style absolute popovers — TextView
 * Find/Goto/Ignore/Format ride the `tv-chrome` overlay stack, so an
 * open panel covers rows instead of pushing them down). Such boxes are
 * excluded from the gate: any pair where EITHER side lives inside a
 * `[data-overlay]` container is skipped. In-flow panels (hex/merge/
 * folder find+goto, folder name/rules) carry NO such marker and are
 * still gated exactly as before. Opt-out is per-element via the
 * `data-overlay` attribute — removing the marker re-arms the gate.
 */
import { expect, type Page } from "@playwright/test";

export const OVERLAP_SELECTORS = "header, nav, main, [data-testid]";

export interface Box {
  readonly sel: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  /** Wave-220: true when the element lives inside a `[data-overlay]` popover. */
  readonly overlay: boolean;
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
        const overlay = el instanceof HTMLElement && el.closest("[data-overlay]") !== null;
        // Orchestrator clip-fix: intersect with the VISIBLE rect (viewport +
        // scroll/clip ancestors). Scrollport children report full content
        // boxes; without clipping, folded rows geometrically "overlap" the
        // footer while pixel-clean. Fixed elements skip ancestor clipping
        // (they escape scroll containers) but still clip to the viewport.
        let vx = r.x;
        let vy = r.y;
        let vw = r.width;
        let vh = r.height;
        const clip = (cx: number, cy: number, cw: number, ch: number) => {
          const x1 = Math.max(vx, cx);
          const y1 = Math.max(vy, cy);
          const x2 = Math.min(vx + vw, cx + cw);
          const y2 = Math.min(vy + vh, cy + ch);
          vx = x1;
          vy = y1;
          vw = Math.max(0, x2 - x1);
          vh = Math.max(0, y2 - y1);
        };
        clip(0, 0, window.innerWidth, window.innerHeight);
        if (el instanceof HTMLElement) {
          const fixed = window.getComputedStyle(el).position === "fixed";
          if (!fixed) {
            let p: HTMLElement | null = el.parentElement;
            while (p) {
              const cs = window.getComputedStyle(p);
              if (cs.overflowX !== "visible" || cs.overflowY !== "visible") {
                const pr = p.getBoundingClientRect();
                clip(pr.x + p.clientLeft, pr.y + p.clientTop, p.clientWidth, p.clientHeight);
              }
              p = p.parentElement;
            }
          }
        }
        return { sel, x: vx, y: vy, width: vw, height: vh, overlay };
      })
      .filter((b) => b.width > 1 && b.height > 1),
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
      // Wave-220: dedicated overlay layer floats above content by design
      // (explicit `[data-overlay]` opt-out). Skip pairs where either side
      // is an overlay or lives inside one — in-flow boxes gate unchanged.
      if (a.overlay || b.overlay) continue;
      if (containsBox(a, b) || containsBox(b, a)) continue;
      if (intersectArea(a, b) > 1) violations.push(`${a.sel} OVERLAPS ${b.sel}`);
    }
  }
  expect(violations, `text overlap: ${violations.slice(0, 10).join("; ")}`).toEqual([]);
}
