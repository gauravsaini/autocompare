/**
 * ODD Phase 3 vertical-slice trace test (mock mode: `VITE_MOCK_IPC=1`).
 *
 * One user click (the Text card) triggers `text.compare`; the test then
 * asserts that a single complete correlated trace exists:
 * - exactly one root span (`parent_span_id: null`, `ts.ui/TextView`)
 * - every span shares that root's `trace_id` and reports `status: ok`
 * - at least one `ts.ipc/client` span proves the IPC round-trip, with
 *   `transport: mock` and `trace_echo: true` (mock echoed our `__trace`)
 * - the root span carries end-of-operation stats (`rows`, `hunks`, `equal`)
 *
 * Span access is read-only via the `window.__AUTOCOMPARE_TRACE__` hook
 * (see `src/ipc/client.ts`); production behavior is unchanged.
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

async function recentTextCompareSpans(page): Promise<WireSpan[]> {
  return page.evaluate(() => {
    const w = window as unknown as {
      __AUTOCOMPARE_TRACE__?: {
        listRecentSpans: (query: unknown) => WireSpan[];
      };
    };
    return (
      w.__AUTOCOMPARE_TRACE__?.listRecentSpans({
        op: "text.compare",
        limit: 50,
      }) ?? []
    );
  });
}

test("text.compare produces one complete correlated trace (mock)", async ({
  page,
}) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  // One user click: open the Text view, which triggers text.compare.
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("v2/home-card-text").click();
  await expect(page.getByTestId("v2/text-view")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible({ timeout: 10000 });

  const spans = await recentTextCompareSpans(page);
  expect(spans.length).toBeGreaterThanOrEqual(2);

  // Group by trace and pick a complete one (mount may emit more than one
  // compare, e.g. under StrictMode double-effects; each must stand alone).
  const byTrace = new Map<string, WireSpan[]>();
  for (const span of spans) {
    const group = byTrace.get(span.trace_id) ?? [];
    group.push(span);
    byTrace.set(span.trace_id, group);
  }
  const complete = [...byTrace.values()].find(
    (group) =>
      group.filter((s) => s.parent_span_id === null).length === 1 &&
      group.every((s) => s.status === "ok") &&
      group.some((s) => s.component === "ts.ui/TextView") &&
      group.some((s) => s.component === "ts.ipc/client"),
  );
  expect(complete).toBeDefined();
  const trace = complete as WireSpan[];
  const traceId = trace[0].trace_id;
  expect(traceId).toMatch(/^[0-9a-f]{32}$/);
  expect(trace.every((s) => s.trace_id === traceId)).toBe(true);

  const root = trace.find((s) => s.parent_span_id === null) as WireSpan;
  expect(root.op).toBe("text.compare");
  expect(root.component).toBe("ts.ui/TextView");
  // Root span ends with row/hunk/equal stats (§5.1 render stats).
  expect(typeof root.attributes?.["rows"]).toBe("number");
  expect(typeof root.attributes?.["hunks"]).toBe("number");
  expect(typeof root.attributes?.["equal"]).toBe("boolean");

  // IPC round-trip proof: mock transport + echoed trace context.
  const ipc = trace.filter((s) => s.component === "ts.ipc/client");
  expect(ipc.length).toBeGreaterThanOrEqual(1);
  expect(
    ipc.some((s) => s.attributes?.["transport"] === "mock"),
  ).toBe(true);
  expect(ipc.some((s) => s.attributes?.["trace_echo"] === true)).toBe(
    true,
  );
});
