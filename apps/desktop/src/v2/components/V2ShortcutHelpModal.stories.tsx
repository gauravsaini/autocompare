import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2ShortcutHelpModal from "./V2ShortcutHelpModal";

const meta = {
  title: "V2/Modals/V2ShortcutHelpModal",
  component: V2ShortcutHelpModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    onClose: fn(),
  },
} satisfies Meta<typeof V2ShortcutHelpModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const AllShortcuts: Story = {};

export const FilteredShortcuts: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const input = canvas.getByPlaceholderText(/Filter shortcuts/i);
    await userEvent.type(input, "merge");
    await expect(canvas.getByText("3-Way Text Merge")).toBeVisible();
  },
};
