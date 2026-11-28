/**
 * ODD Phase 4 Leaf A trace-view tests (mock mode: `VITE_MOCK_IPC=1`).
 *
 * One user click per view (Folder, Table) triggers its compare; the test then
 * asserts that a single complete correlated trace exists:
 * - exactly one root span (`parent_span_id: null`, view component)
 * - every span shares that root's `trace_id` and reports `status: ok`
 * - at least one `ts.ipc/client` span proves the IPC round-trip, with
 *   `transport: mock` and `trace_echo: true` (mock echoed our `__trace`)
 * - the root span carries end-of-operation stats (`rows`, `hunks`, `equal`)
 *
 * Span access is read-only via the `window.__AUTOCOMPARE_TRACE__` hook
 * (see `src/ipc/client.ts`); production behavior is unchanged.
 * Grouping is StrictMode-safe: mount may emit more than one compare, and
 * each must stand alone as a complete trace.
 */
import { expect, test } from "@playwright/test";

interface WireSpan {
  readonly kind: string;
  readonly op: string;
  readonly trace_id: string;
  readonly span_id: string;
  readonly parent_span_id: string | null;
  readonly component: string;
  readonly status: string;
  readonly attributes?: Record<string, unknown>;
}

async function recentSpans(page, op: string): Promise<WireSpan[]> {
  return page.evaluate((wantedOp: string) => {
    const w = window as unknown as {
      __AUTOCOMPARE_TRACE__?: {
        listRecentSpans: (query: unknown) => WireSpan[];
      };
    };
    return (
      w.__AUTOCOMPARE_TRACE__?.listRecentSpans({
        op: wantedOp,
        limit: 50,
      }) ?? []
    );
  }, op);
}

function findCompleteTrace(spans: WireSpan[], rootOp: string, rootComponent: string): WireSpan[] | undefined {
  const byTrace = new Map<string, WireSpan[]>();
  for (const span of spans) {
    const group = byTrace.get(span.trace_id) ?? [];
    group.push(span);
    byTrace.set(span.trace_id, group);
  }
  return [...byTrace.values()].find(
    (group) =>
      group.filter((s) => s.parent_span_id === null).length === 1 &&
      group.every((s) => s.status === "ok") &&
      group.some((s) => s.op === rootOp && s.component === rootComponent) &&
      group.some((s) => s.component === "ts.ipc/client"),
  );
}

test("folder.compare produces one complete correlated trace (mock)", async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  // One user click: open the Folder view, which triggers folder.compare.
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("v2/home-card-folder").click();
  await expect(page.getByTestId("v2/folder-view")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("v2/statusbar")).toBeVisible({ timeout: 10000 });

  await expect
    .poll(async () => (await recentSpans(page, "folder.compare")).length, { timeout: 10000 })
    .toBeGreaterThanOrEqual(2);

  const spans = await recentSpans(page, "folder.compare");
  const trace = findCompleteTrace(spans, "folder.compare", "ts.ui/FolderView");
  expect(trace).toBeDefined();
  const traceId = (trace as WireSpan[])[0].trace_id;
  expect(traceId).toMatch(/^[0-9a-f]{32}$/);
  expect((trace as WireSpan[]).every((s) => s.trace_id === traceId)).toBe(true);

  const root = (trace as WireSpan[]).find((s) => s.parent_span_id === null) as WireSpan;
  expect(root.op).toBe("folder.compare");
  expect(root.component).toBe("ts.ui/FolderView");
  // Root span ends with row/hunk/equal stats (§5.1 render stats).
  expect(typeof root.attributes?.["rows"]).toBe("number");
  expect(typeof root.attributes?.["hunks"]).toBe("number");
  expect(typeof root.attributes?.["equal"]).toBe("boolean");

  // IPC round-trip proof: mock transport + echoed trace context.
  const ipc = (trace as WireSpan[]).filter((s) => s.component === "ts.ipc/client");
  expect(ipc.length).toBeGreaterThanOrEqual(1);
  expect(ipc.some((s) => s.attributes?.["transport"] === "mock")).toBe(true);
  expect(ipc.some((s) => s.attributes?.["trace_echo"] === true)).toBe(true);
});

test("table.compare produces one complete correlated trace (mock)", async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  // One user click: open the Table view, which triggers table.compare.
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("v2/home-card-table").click();
  await expect(page.getByTestId("v2/table-view")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("v2/statusbar")).toBeVisible({ timeout: 10000 });

  await expect
    .poll(async () => (await recentSpans(page, "table.compare")).length, { timeout: 10000 })
    .toBeGreaterThanOrEqual(2);

  const spans = await recentSpans(page, "table.compare");
  const trace = findCompleteTrace(spans, "table.compare", "ts.ui/TableView");
  expect(trace).toBeDefined();
  const traceId = (trace as WireSpan[])[0].trace_id;
  expect(traceId).toMatch(/^[0-9a-f]{32}$/);
  expect((trace as WireSpan[]).every((s) => s.trace_id === traceId)).toBe(true);

  const root = (trace as WireSpan[]).find((s) => s.parent_span_id === null) as WireSpan;
  expect(root.op).toBe("table.compare");
  expect(root.component).toBe("ts.ui/TableView");
  // Root span ends with row/hunk/equal stats (§5.1 render stats).
  expect(typeof root.attributes?.["rows"]).toBe("number");
  expect(typeof root.attributes?.["hunks"]).toBe("number");
  expect(typeof root.attributes?.["equal"]).toBe("boolean");

  // IPC round-trip proof: mock transport + echoed trace context.
  const ipc = (trace as WireSpan[]).filter((s) => s.component === "ts.ipc/client");
  expect(ipc.length).toBeGreaterThanOrEqual(1);
  expect(ipc.some((s) => s.attributes?.["transport"] === "mock")).toBe(true);
  expect(ipc.some((s) => s.attributes?.["trace_echo"] === true)).toBe(true);
});
