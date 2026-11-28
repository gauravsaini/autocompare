import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2SettingsModal from "./V2SettingsModal";

const meta = {
  title: "V2/Modals/V2SettingsModal",
  component: V2SettingsModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    theme: "light",
    onThemeChange: fn(),
    onClose: fn(),
  },
} satisfies Meta<typeof V2SettingsModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "dark" }));
    await expect(args.onThemeChange).toHaveBeenCalledWith("dark");
    await userEvent.click(canvas.getByRole("button", { name: /Check for Updates/i }));
    await expect(canvas.getByText(/browser preview/i)).toBeVisible();
  },
};

export const DarkAppearanceSelected: Story = {
  args: {
    theme: "dark",
  },
};
