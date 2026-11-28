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

test("tracer overhead: telemetry on vs off delta within §11 budget (offline)", async () => {
  // ODD Phase 12 overhead gate (§11): telemetry ON vs OFF wall-clock delta on
  // a representative CPU workload must stay within the p99 <3% added-latency
  // budget, and mean per-op tracer cost must stay under the <1ms/op UI-thread
  // budget. Node-side microbench of the bundled TS tracer (no browser timing
  // involved); interleaved A/B rounds with medians collapse GC/drift noise.
  // Additive-only: touches no existing test, fixture, or app logic.
  const tracer = await import("../../src/observability/tracer.ts");
  const sink = await import("../../src/observability/sink.ts");

  const ITER = 20; // spans per round (one span ≈ one traced op)
  const WORK = 600_000; // busy-loop iterations ≈ stand-in op body (ms scale)
  const ROUNDS = 5;

  let blackhole = 0;
  function workload(): void {
    let acc = 0;
    for (let i = 0; i < WORK; i += 1) acc = (acc + i) % 100_003;
    blackhole = (blackhole + acc) % 100_003;
  }

  function roundSeconds(telemetryOn: boolean): number {
    tracer.setTelemetryEnabled(telemetryOn);
    sink.clearBuffer();
    const start = performance.now();
    for (let i = 0; i < ITER; i += 1) {
      if (!telemetryOn) {
        workload();
        continue;
      }
      const span = tracer.startRootSpan({
        op: "perf.budget_probe",
        component: "ts.perf/probe",
      });
      workload();
      span.set({ iters: WORK });
      tracer.endSpan(span);
    }
    const elapsed = performance.now() - start;
    if (telemetryOn) {
      // Sanity: telemetry really was on — one span per op landed in the ring.
      expect(sink.getBufferedRecords().length).toBe(ITER);
    }
    return elapsed / 1000;
  }

  function median(values: number[]): number {
    const sorted = [...values].sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    if (sorted.length % 2 === 1) return sorted[mid] as number;
    return ((sorted[mid - 1] as number) + (sorted[mid] as number)) / 2;
  }

  try {
    const offSecs: number[] = [];
    const onSecs: number[] = [];
    for (let r = 0; r < ROUNDS; r += 1) {
      offSecs.push(roundSeconds(false));
      onSecs.push(roundSeconds(true));
    }
    const off = median(offSecs);
    const on = median(onSecs);
    const delta = Math.max(0, on - off);

    // §11: added latency <3% of the operation wall time.
    expect(off).toBeGreaterThan(0);
    expect(delta).toBeLessThan(off * 0.03);
    // §11: UI-thread tracing <1ms per op (mean over the round).
    expect(delta / ITER).toBeLessThan(0.001);
  } finally {
    tracer.setTelemetryEnabled(true);
    sink.clearBuffer();
  }
  expect(blackhole).toBeGreaterThanOrEqual(0);
});
