import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2NewSessionModal from "./V2NewSessionModal";

const meta = {
  title: "V2/Modals/V2NewSessionModal",
  component: V2NewSessionModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    onClose: fn(),
    onCreate: fn(),
  },
} satisfies Meta<typeof V2NewSessionModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const TenPluginGrid: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/new-session-card-structured"));
    await userEvent.click(canvas.getByTestId("v2/new-session-open-btn"));
    await expect(args.onCreate).toHaveBeenCalledWith("structured", "", "");
    await expect(args.onClose).toHaveBeenCalled();
  },
};

export const FilteredSearch: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const input = canvas.getByPlaceholderText(/Filter comparators/i);
    await userEvent.type(input, "toml");
    await expect(canvas.getByTestId("v2/new-session-card-toml-xml")).toBeVisible();
  },
};
