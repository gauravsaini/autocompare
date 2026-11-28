import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import V2SyncCompare from "./V2SyncCompare";

const meta = {
  title: "V2/Views/V2SyncCompare",
  component: V2SyncCompare,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    leftPath: "",
    rightPath: "",
    onPathsChange: fn(),
    onRegisterCommands: fn(),
    onStatusChange: fn(),
  },
} satisfies Meta<typeof V2SyncCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultPlan: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/sync-view", {}, { timeout: 5000 }),
    ).toBeVisible();
    await waitFor(
      async () => {
        await expect(canvas.getByRole("button", { name: "Dry Run" })).toBeEnabled();
      },
      { timeout: 5000 },
    );
  },
};

export const ExcludedRowAndDryRun: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/sync-view", {}, { timeout: 5000 }),
    ).toBeVisible();
    const checkboxes = await canvas.findAllByRole(
      "checkbox",
      { name: /^Include / },
      { timeout: 5000 },
    );
    await userEvent.click(checkboxes[0]);
    await expect(canvas.getByText("Excluded: 1")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Dry Run" }));
    await expect(
      await canvas.findByText(/Dry-Run Preview:/i, {}, { timeout: 5000 }),
    ).toBeVisible();
  },
};
