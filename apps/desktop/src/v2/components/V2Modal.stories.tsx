import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2Modal from "./V2Modal";

const meta = {
  title: "V2/Modals/V2Modal",
  component: V2Modal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    title: "Session Options",
    subtitle: "Configure comparison defaults and layout preferences",
    onClose: fn(),
    children: (
      <div className="space-y-3 text-[12px] text-[var(--v2-fg-secondary)]">
        <p>Modal content area with centered viewport backdrop and Esc dismissal.</p>
        <div className="rounded-lg border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] p-3">
          Preview panel inside modal container.
        </div>
      </div>
    ),
    footer: (
      <>
        <button type="button" className="v2-btn">
          Cancel
        </button>
        <button type="button" className="v2-btn v2-btn-primary">
          Apply
        </button>
      </>
    ),
  },
} satisfies Meta<typeof V2Modal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("dialog")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Close modal" }));
    await expect(args.onClose).toHaveBeenCalledTimes(1);
  },
};

export const WideNoFooter: Story = {
  args: {
    widthClass: "max-w-3xl",
    footer: undefined,
  },
};
