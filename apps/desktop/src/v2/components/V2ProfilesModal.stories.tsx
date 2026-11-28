import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2ProfilesModal from "./V2ProfilesModal";

const meta = {
  title: "V2/Modals/V2ProfilesModal",
  component: V2ProfilesModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    onClose: fn(),
    onSaved: fn(),
  },
} satisfies Meta<typeof V2ProfilesModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const GlobalTab: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByTestId("v2/profiles-modal")).toBeVisible();
    await userEvent.click(canvas.getByTitle("New profile"));
    await expect(canvas.getByDisplayValue("Profile 1")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(args.onSaved).toHaveBeenCalled();
  },
};

export const VfsSchemesTab: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "VFS Schemes" }));
    await expect(canvas.getByText("zip://")).toBeVisible();
  },
};

export const ConnectionTab: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Connection" }));
    await expect(canvas.getByPlaceholderText("Optional PEM path")).toBeVisible();
  },
};
