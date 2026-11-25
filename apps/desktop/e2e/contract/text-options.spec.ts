/**
 * Contract Spec: Minor / Unimportant, Suppressed Lines, Encoding, and EOL (Wave-L1).
 *
 * Verifies the feature contract across engine + IPC + mock:
 *  1. Option in -> echoed appliedOptions:
 *     - minorUnimportant / minor_unimportant
 *     - encoding ("utf-8", "windows-1252", etc.)
 *     - eol ("lf", "crlf")
 *     - suppressedLines / suppressed_lines
 *  2. Suppressed rows flagged, NOT dropped:
 *     - Row indices in suppressedLines have `suppressed: true`
 *     - Total row count remains intact (no rows dropped from page)
 *  3. Both camelCase and snake_case wire spellings accepted and honored.
 *  4. Backward compatibility: default compare without flags leaves appliedOptions clean.
 */
import { expect, test } from "@playwright/test";

test.describe("Text Contract L1: Minor / Format / Suppressed / Encoding / EOL", () => {
  test.beforeEach(async ({ page }) => {
    let reachable = false;
    try {
      await page.goto("/", { timeout: 15000 });
      reachable = true;
    } catch {
      reachable = false;
    }
    test.skip(!reachable, "dev server unreachable -- skipping contract spec");
  });

  test("1. option in -> echoed appliedOptions for camelCase flags", async ({ page }) => {
    const res = await page.evaluate(async () => {
      const { textCompare } = await import("../../src/ipc/text");
      return textCompare("left.txt", "right.txt", {
        minorUnimportant: true,
        encoding: "windows-1252",
        eol: "crlf",
        suppressedLines: [1, 3],
      });
    });

    expect(res).toBeDefined();
    expect(res.appliedOptions).toBeDefined();
    expect(res.appliedOptions?.minorUnimportant).toBe(true);
    expect(res.appliedOptions?.encoding).toBe("windows-1252");
    expect(res.appliedOptions?.eol).toBe("crlf");
    expect(res.appliedOptions?.suppressedLines).toEqual([1, 3]);
  });

  test("2. option in -> echoed appliedOptions for snake_case flags", async ({ page }) => {
    const res = await page.evaluate(async () => {
      const { textCompare } = await import("../../src/ipc/text");
      return textCompare("left.txt", "right.txt", {
        minor_unimportant: true,
        encoding: "utf-8",
        eol: "lf",
        suppressed_lines: [0, 2],
      });
    });

    expect(res).toBeDefined();
    expect(res.appliedOptions).toBeDefined();
    expect(res.appliedOptions?.minorUnimportant).toBe(true);
    expect(res.appliedOptions?.encoding).toBe("utf-8");
    expect(res.appliedOptions?.eol).toBe("lf");
    expect(res.appliedOptions?.suppressedLines).toEqual([0, 2]);
  });

  test("3. suppressed rows are flagged and NOT dropped", async ({ page }) => {
    const data = await page.evaluate(async () => {
      const { textCompare, textGetRows } = await import("../../src/ipc/text");
      const comp = await textCompare("left.txt", "right.txt", {
        suppressedLines: [1, 4],
      });
      const pageRows = await textGetRows(comp.session, 0, 10);
      return { comp, pageRows };
    });

    const { comp, pageRows } = data;
    expect(comp.rows).toBeGreaterThan(0);
    // Suppressed lines do NOT drop rows: exactly 10 rows returned for start=0, limit=10
    expect(pageRows.rows.length).toBe(10);

    // Row 1 and 4 flagged as suppressed
    expect(pageRows.rows[1].suppressed).toBe(true);
    expect(pageRows.rows[4].suppressed).toBe(true);

    // Other rows are not suppressed
    expect(pageRows.rows[0].suppressed).toBeFalsy();
    expect(pageRows.rows[2].suppressed).toBeFalsy();
    expect(pageRows.rows[3].suppressed).toBeFalsy();
    expect(pageRows.rows[5].suppressed).toBeFalsy();
  });

  test("4. raw mock invoke parses both spellings and passthroughs encoding/EOL", async ({ page }) => {
    const data = await page.evaluate(async () => {
      const { invoke } = await import("../../src/ipc/client");
      const comp = await invoke<Record<string, unknown>>("text_compare", {
        left: "/path/a.txt",
        right: "/path/b.txt",
        minor_unimportant: true,
        encoding: "shift_jis",
        eol: "crlf",
        suppressed_lines: [2],
      });
      const pageRows = await invoke<{ rows: Array<{ index: number; suppressed?: boolean }> }>(
        "text_get_rows",
        { session: comp.session, start: 0, limit: 5 },
      );
      return { comp, pageRows };
    });

    const applied = data.comp.appliedOptions as Record<string, unknown>;
    expect(applied).toBeDefined();
    expect(applied.minorUnimportant).toBe(true);
    expect(applied.encoding).toBe("shift_jis");
    expect(applied.eol).toBe("crlf");
    expect(applied.suppressedLines).toEqual([2]);

    expect(data.pageRows.rows.length).toBe(5);
    expect(data.pageRows.rows[2].suppressed).toBe(true);
    expect(data.pageRows.rows[0].suppressed).toBeFalsy();
    expect(data.pageRows.rows[1].suppressed).toBeFalsy();
  });

  test("5. backward compatibility: default compare without flags leaves appliedOptions clean", async ({ page }) => {
    const res = await page.evaluate(async () => {
      const { textCompare } = await import("../../src/ipc/text");
      return textCompare("left.txt", "right.txt");
    });

    expect(res).toBeDefined();
    // Legacy invocation omits appliedOptions completely
    expect(res.appliedOptions).toBeUndefined();
    expect(res.rows).toBeGreaterThan(0);
  });

  test("6. audioCompare echoes numeric tolerance argument round-trip", async ({ page }) => {
    const res = await page.evaluate(async () => {
      const { audioCompare } = await import("../../src/ipc/media");
      const explicit = await audioCompare("left.wav", "right.wav", 0.05);
      const def = await audioCompare("left.wav", "right.wav");
      return { explicit, def };
    });

    expect(res.explicit).toBeDefined();
    expect(res.explicit.tolerance).toBe(0.05);
    expect(res.def).toBeDefined();
    expect(res.def.tolerance).toBe(0);
  });

  test("7. syncPlan honors direction option round-trip", async ({ page }) => {
    const res = await page.evaluate(async () => {
      const { syncPlan } = await import("../../src/ipc/sync");
      const defaultPlan = await syncPlan("/path/left", "/path/right");
      const rtlPlan = await syncPlan("/path/left", "/path/right", {
        direction: "RightToLeft",
      });
      return { defaultPlan, rtlPlan };
    });

    expect(res.defaultPlan).toBeDefined();
    expect(res.rtlPlan).toBeDefined();
    expect(res.rtlPlan.steps.length).toBeGreaterThan(0);
    const rtlActions = res.rtlPlan.steps.filter(
      (s) => s.action === "CopyToLeft" || s.action === "DeleteLeft",
    );
    expect(rtlActions.length).toBeGreaterThan(0);
  });
});
