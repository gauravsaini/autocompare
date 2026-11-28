import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2FolderCompare from "./V2FolderCompare";

const meta = {
  title: "V2/Views/V2FolderCompare",
  component: V2FolderCompare,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    leftPath: "",
    rightPath: "",
    onPathsChange: fn(),
    onOpenTextPair: fn(),
    onRegisterCommands: fn(),
    onStatusChange: fn(),
  },
} satisfies Meta<typeof V2FolderCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultTree: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/folder-view")).toBeVisible();
    await expect(await canvas.findByText("d00")).toBeVisible();
  },
};

export const EntrySelectedActionBar: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const entry = await canvas.findByText("d00");
    await userEvent.click(entry);
    await expect(await canvas.findByRole("button", { name: /Copy to Right/i })).toBeVisible();
  },
};
