import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2ConfirmModal from "./V2ConfirmModal";

const meta = {
  title: "V2/Modals/V2ConfirmModal",
  component: V2ConfirmModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    title: "Restore Factory Defaults",
    subtitle: "Reset File Formats and VFS Profiles",
    message:
      "Restore factory default File Formats and VFS Profiles? Any custom format rules or profiles will be overwritten.",
    confirmLabel: "Restore Defaults",
    cancelLabel: "Cancel",
    destructive: false,
    onConfirm: fn(),
    onCancel: fn(),
  },
} satisfies Meta<typeof V2ConfirmModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Standard: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/confirm-accept"));
    await expect(args.onConfirm).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByTestId("v2/confirm-cancel"));
    await expect(args.onCancel).toHaveBeenCalledTimes(1);
  },
};

export const Destructive: Story = {
  args: {
    title: "Delete Item",
    subtitle: "right side",
    message: 'Delete "src/legacy-config.json" on the right side? This cannot be undone.',
    confirmLabel: "Delete",
    destructive: true,
  },
};
