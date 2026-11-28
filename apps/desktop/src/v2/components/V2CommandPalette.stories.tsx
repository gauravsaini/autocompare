import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2CommandPalette, { type PaletteCommand } from "./V2CommandPalette";

const SAMPLE_COMMANDS: readonly PaletteCommand[] = [
  { id: "new-text", category: "Compare", label: "New Text Compare", shortcut: "⌘1", run: fn() },
  { id: "new-folder", category: "Compare", label: "New Folder Compare", shortcut: "⌘2", run: fn() },
  { id: "new-hex", category: "Compare", label: "New Hex Compare", shortcut: "⌘3", run: fn() },
  { id: "new-structured", category: "Compare", label: "New JSON / YAML Structural Diff", run: fn() },
  { id: "toggle-theme", category: "View", label: "Switch to Dark Mode", run: fn() },
  { id: "open-settings", category: "App", label: "Open Preferences", shortcut: "⌘,", run: fn() },
];

const meta = {
  title: "V2/Modals/V2CommandPalette",
  component: V2CommandPalette,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    commands: SAMPLE_COMMANDS,
    onClose: fn(),
  },
} satisfies Meta<typeof V2CommandPalette>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const input = canvas.getByPlaceholderText("Type a command or comparison mode…");
    await userEvent.type(input, "Hex");
    await expect(canvas.getByText("New Hex Compare")).toBeVisible();
    await userEvent.click(canvas.getByText("New Hex Compare"));
    await expect(args.onClose).toHaveBeenCalledTimes(1);
    await expect(SAMPLE_COMMANDS[2].run).toHaveBeenCalledTimes(1);
  },
};

export const EmptyNoMatches: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const input = canvas.getByPlaceholderText("Type a command or comparison mode…");
    await userEvent.type(input, "nonexistent-command");
    await expect(canvas.getByText("No matching commands")).toBeVisible();
  },
};
