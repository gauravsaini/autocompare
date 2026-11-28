import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
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
    await expect(await canvas.findByTestId("v2/sync-view")).toBeVisible();
    await expect(await canvas.findByRole("button", { name: "Dry Run" })).toBeEnabled();
  },
};

export const ExcludedRowAndDryRun: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/sync-view")).toBeVisible();
    const checkboxes = await canvas.findAllByRole("checkbox", { name: /^Include / });
    await userEvent.click(checkboxes[0]);
    await expect(canvas.getByText("Excluded: 1")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Dry Run" }));
    await expect(await canvas.findByText(/Dry-Run Preview:/i)).toBeVisible();
  },
};
