import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2FolderMerge from "./V2FolderMerge";

const meta = {
  title: "V2/Views/V2FolderMerge",
  component: V2FolderMerge,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    leftPath: "",
    rightPath: "",
    onPathsChange: fn(),
    onOpenTextMerge: fn(),
    onRegisterCommands: fn(),
    onStatusChange: fn(),
  },
} satisfies Meta<typeof V2FolderMerge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultTreeMergePlan: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/foldermerge-view")).toBeVisible();
    await expect(await canvas.findByRole("button", { name: /Conflicts → Left/i })).toBeVisible();
  },
};

export const ConflictsResolvedToLeft: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const bulkBtn = await canvas.findByRole("button", { name: /Conflicts → Left/i });
    await userEvent.click(bulkBtn);
    await expect(canvas.queryByRole("button", { name: /Conflicts → Left/i })).toBeNull();
  },
};
