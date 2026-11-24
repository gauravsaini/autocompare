/**
 * Wave-26 pathbar-history proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * The PathBar owns a Beyond Compare style per-pane recent-path picker:
 * each path input gets a history chevron; submitting a path (Enter)
 * records it at top (dedup, max 8, `localStorage` persisted); the
 * dropdown lists entries, click fills the input WITHOUT auto-submitting,
 * Escape closes, and an empty history shows an honest empty note.
 *
 * This spec drives the REAL `App` workspace (no session needed: submit
 * records history locally, the compare flow is untouched) and proves:
 *  1. empty history shows the empty note; Escape closes the dropdown,
 *  2. submit -> reopen shows the entry; picking refills the input and
 *     closes without recording a duplicate,
 *  3. histories are per-side, deduped, capped at 8, and survive reload,
 *  4. light + dark PNGs land in `/tmp/ui-26/` (never in-repo),
 *  5. zero console/page errors throughout.
 *
 * Wave-181 hardening (flake fix): the App boots async AFTER first paint —
 * the bundled demo auto-open (`textCompareDemo` -> `openSession`) and the
 * session-restore hydration push new `leftPath`/`rightPath` props, and the
 * PathBar prop-sync effect adopts them by overwriting the input drafts.
 * The old `resetHistory` (`localStorage.clear()` + reload) made this worse:
 * wiping `autocompare:mockSessionStore:v1` guarantees the demo fires on
 * every reload, so a fill -> Enter sequence could land exactly while the
 * boot props arrived and the draft came back as
 * `tools/parity-harness/fixtures/t_left.txt`. Fix, spec-side only:
 *  - reset removes ONLY `ac:pathbar-history:*` keys (sessions survive, so
 *    the demo fires at most once per test and restore re-hydrates the same
 *    paths);
 *  - after every reload, `waitStableInputs` polls both drafts until they
 *    stop changing (consecutive identical samples) instead of assuming a
 *    fixed sleep is enough — pure retry, exits early, loud on timeout;
 *  - `openHistory` is idempotent (no double-toggle close);
 *  - the dark-mode `waitForTimeout(300)` became a computed-style poll.
 *
 * Stable-testids-only rule: every locator below is owned by wave-26
 * (`pathbar/<side>`, `pathbar/<side>-history-toggle`,
 * `pathbar/<side>-history`, `pathbar/<side>-history-empty`,
 * `pathbar/<side>-history-item`), all defined in
 * `apps/desktop/src/components/chrome/PathBar.tsx`. Skip policy for
 * infra (dev server unreachable, browsers missing offline) matches the
 * other specs.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-26";
const LEFT_A = "/tmp/hist-left-a.txt";
const LEFT_B = "/tmp/hist-left-b.txt";
const RIGHT_A = "/tmp/hist-right-a.txt";

/** `localStorage` prefix owned by PathBar history (see PathBar.tsx). */
const HISTORY_PREFIX = "ac:pathbar-history:";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("pathbar")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

/** Fresh console/page-error collector; call `assertNoErrors` at test end. */
function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(String(err)));
  return errors;
}

/**
 * Wave-181: wait until the App boot stops rewriting the PathBar drafts.
 * Demo auto-open + session restore push props async after first paint;
 * sample both inputs until consecutive reads agree (retry only, no fixed
 * sleep assumption). Resolves fast when already settled; throws loudly on
 * timeout instead of letting a later fill race the boot.
 */
async function waitStableInputs(page: Page, timeoutMs = 20000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let last = "\u0000";
  let steady = 0;
  for (;;) {
    const left = await page.getByTestId("pathbar/left").inputValue();
    const right = await page.getByTestId("pathbar/right").inputValue();
    const snap = `${left}\n${right}`;
    steady = snap === last ? steady + 1 : 0;
    last = snap;
    if (steady >= 3) return;
    if (Date.now() > deadline) {
      throw new Error(`boot inputs never settled (last=${JSON.stringify(last)})`);
    }
    await page.waitForTimeout(100);
  }
}

/**
 * Deterministic history reset: drop ONLY the PathBar history keys, keep
 * the mock session store + last-active id so the demo boot does not refire
 * and restore re-hydrates identical props; then settle the boot writes.
 */
async function resetHistory(page: Page): Promise<void> {
  await page.evaluate((prefix) => {
    const doomed: string[] = [];
    for (let i = 0; i < window.localStorage.length; i += 1) {
      const key = window.localStorage.key(i);
      if (key !== null && key.startsWith(prefix)) doomed.push(key);
    }
    for (const key of doomed) window.localStorage.removeItem(key);
  }, HISTORY_PREFIX);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("pathbar")).toBeVisible({ timeout: 15000 });
  await waitStableInputs(page);
}

/** Reload that preserves history but re-settles async boot/restore writes. */
async function reloadAndSettle(page: Page): Promise<void> {
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("pathbar")).toBeVisible({ timeout: 15000 });
  await waitStableInputs(page);
}

async function submitPath(page: Page, side: "left" | "right", path: string): Promise<void> {
  await page.getByTestId(`pathbar/${side}`).fill(path);
  await page.getByTestId(`pathbar/${side}`).press("Enter");
  await expect(page.getByTestId(`pathbar/${side}`)).toHaveValue(path);
}

/** Idempotent open: never double-toggle an already-open dropdown shut. */
async function openHistory(page: Page, side: "left" | "right"): Promise<void> {
  const dropdown = page.getByTestId(`pathbar/${side}-history`);
  if ((await dropdown.count()) === 0) {
    // Native disabled while comparing: click auto-waits for enabled.
    await page.getByTestId(`pathbar/${side}-history-toggle`).click();
  }
  await expect(dropdown).toBeVisible();
}

test("empty history shows honest note; Escape closes without layout breakage", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await resetHistory(page);

  // Closed dropdown renders nothing into layout.
  await expect(page.getByTestId("pathbar/left-history")).toHaveCount(0);
  await expect(page.getByTestId("pathbar/right-history")).toHaveCount(0);

  // Empty history shows the honest empty note.
  await openHistory(page, "left");
  await expect(page.getByTestId("pathbar/left-history-empty")).toContainText(
    "No recent paths yet",
  );

  // Escape closes; the bar itself is undisturbed.
  await page.getByTestId("pathbar/left").press("Escape");
  await expect(page.getByTestId("pathbar/left-history")).toBeHidden();
  await expect(page.getByTestId("pathbar")).toBeVisible();

  expect(errors, "zero console/page errors").toEqual([]);
});

test("submit records; reopen lists; pick fills without auto-submit or duplicate", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await resetHistory(page);

  // Submit records at top.
  await submitPath(page, "left", LEFT_A);
  await openHistory(page, "left");
  await expect(page.getByTestId("pathbar/left-history-item")).toHaveCount(1);
  await expect(page.getByTestId("pathbar/left-history-item").first()).toContainText(LEFT_A);

  // Diverge the input, then pick: input refills, dropdown closes,
  // and picking records nothing (still exactly one entry on reopen).
  await page.getByTestId("pathbar/left").fill("/tmp/scratch.txt");
  await page.getByTestId("pathbar/left-history-item").first().click();
  await expect(page.getByTestId("pathbar/left-history")).toBeHidden();
  await expect(page.getByTestId("pathbar/left")).toHaveValue(LEFT_A);
  await openHistory(page, "left");
  await expect(page.getByTestId("pathbar/left-history-item")).toHaveCount(1);

  // aria wiring stays honest.
  await expect(page.getByTestId("pathbar/left-history-toggle")).toHaveAttribute(
    "aria-expanded",
    "true",
  );

  expect(errors, "zero console/page errors").toEqual([]);
});

test("per-side isolation, dedup, cap-8, reload persistence, light+dark", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await resetHistory(page);

  // Per-side: a right submit never leaks into the left list.
  await submitPath(page, "right", RIGHT_A);
  await openHistory(page, "right");
  await expect(page.getByTestId("pathbar/right-history-item").first()).toContainText(RIGHT_A);
  await page.getByTestId("pathbar/left").press("Escape");
  await openHistory(page, "left");
  await expect(page.getByTestId("pathbar/left-history")).not.toContainText(RIGHT_A);
  await expect(page.getByTestId("pathbar/left-history-empty")).toBeVisible();
  await page.getByTestId("pathbar/left").press("Escape");

  // Order + dedup: newest first, re-submit moves to top without dupes.
  await submitPath(page, "left", LEFT_A);
  await submitPath(page, "left", LEFT_B);
  await submitPath(page, "left", LEFT_A);
  await openHistory(page, "left");
  const items = page.getByTestId("pathbar/left-history-item");
  await expect(items).toHaveCount(2);
  await expect(items.nth(0)).toContainText(LEFT_A);
  await expect(items.nth(1)).toContainText(LEFT_B);
  await page.getByTestId("pathbar/left").press("Escape");

  // Cap: 9 distinct submits keep only the newest 8.
  for (let i = 1; i <= 9; i += 1) {
    // eslint-disable-next-line no-await-in-loop
    await submitPath(page, "left", `/tmp/cap-${i}.txt`);
  }
  await openHistory(page, "left");
  await expect(page.getByTestId("pathbar/left-history-item")).toHaveCount(8);
  await expect(page.getByTestId("pathbar/left-history")).toContainText("/tmp/cap-9.txt");
  await expect(page.getByTestId("pathbar/left-history")).not.toContainText("/tmp/cap-1.txt");
  await page.getByTestId("pathbar/left").press("Escape");

  // Persistence: reload keeps the newest entry on top.
  await reloadAndSettle(page);
  await openHistory(page, "left");
  await expect(page.getByTestId("pathbar/left-history-item").first()).toContainText(
    "/tmp/cap-9.txt",
  );

  // Light + dark evidence (never in-repo). Dark flip is a
  // `prefers-color-scheme` token swap: snapshot the light background, then
  // poll until the computed style actually changes (no fixed sleep).
  await page.screenshot({ path: `${EVID_DIR}/pathbar-history-light.png` });
  const lightBg = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--ac-bg").trim(),
  );
  await page.emulateMedia({ colorScheme: "dark" });
  await expect
    .poll(
      async () =>
        page.evaluate(() =>
          getComputedStyle(document.documentElement).getPropertyValue("--ac-bg").trim(),
        ),
      { timeout: 10000 },
    )
    .not.toBe(lightBg);
  await page.screenshot({ path: `${EVID_DIR}/pathbar-history-dark.png` });

  expect(errors, "zero console/page errors").toEqual([]);
});
