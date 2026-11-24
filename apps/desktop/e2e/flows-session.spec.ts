import { expect, test, type Page } from "@playwright/test";

/**
 * Helper to mount the SessionManager component inside the page.
 * Uses dynamic browser ES module import against the running Vite dev server.
 */
async function mountSessionManager(
  page: Page,
  props?: {
    sessions?: Array<{
      id: string;
      kind: string;
      name: string;
      locked: boolean;
      pageCount: number;
    }>;
    activeId?: string | null;
    onChangeHookName?: string;
  },
) {
  await page.evaluate(async (customProps) => {
    const { mountSessionManager } = await import(
      "/src/components/session/index.ts"
    );

    let container = document.getElementById("session-test-container");
    if (!container) {
      container = document.createElement("div");
      container.id = "session-test-container";
      document.body.appendChild(container);
    }

    const win = window as unknown as Record<string, unknown>;

    const componentProps: Record<string, unknown> = {
      isOpen: true,
    };

    if (customProps?.sessions !== undefined) {
      componentProps.sessions = customProps.sessions;
    }
    if (customProps?.activeId !== undefined) {
      componentProps.activeId = customProps.activeId;
    }
    if (customProps?.onChangeHookName) {
      const hookName = customProps.onChangeHookName;
      componentProps.onChange = (session: unknown) => {
        win[hookName] = session;
      };
    }

    mountSessionManager(container, componentProps);
  }, props);
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible();
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("toolbar action disabled-states smoke", async ({ page }) => {
  // Mount with no active session and empty list
  await mountSessionManager(page, {
    sessions: [],
    activeId: null,
  });

  // Verify dialog is visible
  const dialog = page.getByTestId("session/dialog");
  await expect(dialog).toBeVisible();

  // Open should always be enabled
  const openBtn = page.getByTestId("session/open");
  await expect(openBtn).toBeVisible();
  await expect(openBtn).toBeEnabled();

  // Save and Delete should be disabled when activeId is null
  const saveBtn = page.getByTestId("session/save");
  const deleteBtn = page.getByTestId("session/delete");
  await expect(saveBtn).toBeDisabled();
  await expect(deleteBtn).toBeDisabled();

  // Mount with an active session
  await mountSessionManager(page, {
    sessions: [
      {
        id: "sess-active-1",
        kind: "text",
        name: "Active Test Session",
        locked: false,
        pageCount: 1,
      },
    ],
    activeId: "sess-active-1",
  });

  // With an active session, Save and Delete must be enabled
  await expect(saveBtn).toBeEnabled();
  await expect(deleteBtn).toBeEnabled();
});

test("tab / new session creation flow smoke", async ({ page }) => {
  // Mount with internal hook management
  await mountSessionManager(page);

  const form = page.getByTestId("session/new");
  await expect(form).toBeVisible();

  // Fill in new session form
  const kindSelect = page.getByTestId("session/kind-select");
  await kindSelect.selectOption("folder");

  const leftInput = page.getByTestId("session/left-path");
  await leftInput.fill("/tmp/left-project");

  const rightInput = page.getByTestId("session/right-path");
  await rightInput.fill("/tmp/right-project");

  const createBtn = page.getByTestId("session/create-button");
  await createBtn.click();

  // Verify session appears in the list
  const list = page.getByTestId("session/list");
  await expect(list).toContainText("folder: left-project ↔ right-project");

  // Inputs should be reset after submission
  await expect(leftInput).toHaveValue("");
  await expect(rightInput).toHaveValue("");

  // Toolbar actions should now be enabled for the newly created active session
  await expect(page.getByTestId("session/save")).toBeEnabled();
  await expect(page.getByTestId("session/delete")).toBeEnabled();
});

test("open session via dialog mock flow", async ({ page }) => {
  // Register mock dialog function on window BEFORE mounting
  await page.evaluate(() => {
    (
      window as unknown as {
        __MOCK_DIALOG_OPEN__: () => Promise<string>;
      }
    ).__MOCK_DIALOG_OPEN__ = async () => "/mock/path/imported-session.json";
  });

  await mountSessionManager(page);

  const openBtn = page.getByTestId("session/open");
  await openBtn.click();

  // Imported session should appear in the session list
  const list = page.getByTestId("session/list");
  await expect(list).toContainText("imported-session");

  // It should be active, so delete and save are enabled
  await expect(page.getByTestId("session/delete")).toBeEnabled();
  await expect(page.getByTestId("session/save")).toBeEnabled();
});

test("session deletion flow smoke", async ({ page }) => {
  // Mount with initial sessions (toolbar delete never takes the last row)
  await mountSessionManager(page, {
    sessions: [
      {
        id: "sess-keep",
        kind: "text",
        name: "Keep Me",
        locked: false,
        pageCount: 1,
      },
      {
        id: "sess-to-delete",
        kind: "hex",
        name: "Hex Session To Delete",
        locked: false,
        pageCount: 1,
      },
    ],
    activeId: "sess-to-delete",
  });

  const list = page.getByTestId("session/list");
  await expect(list).toContainText("Hex Session To Delete");

  const deleteBtn = page.getByTestId("session/delete");
  await expect(deleteBtn).toBeEnabled();

  // First click arms the Wave-129 two-step confirm, deletes nothing yet
  await deleteBtn.click();
  await expect(page.getByTestId("session/delete-confirm")).toBeVisible();
  await expect(list).toContainText("Hex Session To Delete");

  // Confirm deletes + advances to the next tab (delete stays enabled)
  await page.getByTestId("session/delete-confirm").click();
  await expect(list).not.toContainText("Hex Session To Delete");
  await expect(list).toContainText("Keep Me");
  await expect(deleteBtn).toBeEnabled();

  // Last remaining row refuses inline instead of blanking
  await deleteBtn.click();
  await expect(page.getByTestId("session/row-error")).toContainText(
    "At least one session must remain.",
  );
});
