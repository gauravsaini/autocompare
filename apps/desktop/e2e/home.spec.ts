import { expect, test } from "@playwright/test";

test("home boots to demo first-paint, Start blank returns to welcome", async ({ page }) => {
  await page.goto("/");
  // Wave-11: wave-10 demo first-paint auto-opens the demo text pair.
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await expect(page.getByTestId("workspace/demo-banner")).toContainText("Demo");
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  // Start blank clears the demo back to the empty welcome state.
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await expect(page.getByTestId("tab-home")).toBeVisible();
});
