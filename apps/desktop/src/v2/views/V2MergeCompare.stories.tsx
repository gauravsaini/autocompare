import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2MergeCompare from "./V2MergeCompare";

const meta = {
  title: "V2/Views/V2MergeCompare",
  component: V2MergeCompare,
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
} satisfies Meta<typeof V2MergeCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const UnresolvedConflict: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("Conflict #1")).toBeVisible();
    await expect(canvas.getByText("Unresolved")).toBeVisible();
  },
};

export const ResolvedWithCustomEdit: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("Conflict #1")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Take Left" }));
    await expect(canvas.getByText("Resolved")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: /Edit Inline/i }));
    await expect(canvas.getByText("Custom resolution for Conflict #1")).toBeVisible();
  },
};
