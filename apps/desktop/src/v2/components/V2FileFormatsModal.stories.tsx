import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2FileFormatsModal from "./V2FileFormatsModal";

const meta = {
  title: "V2/Modals/V2FileFormatsModal",
  component: V2FileFormatsModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    onClose: fn(),
    onSaved: fn(),
  },
} satisfies Meta<typeof V2FileFormatsModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultFormats: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByTestId("v2/file-formats-modal")).toBeVisible();
    await userEvent.click(canvas.getByTitle("Add format"));
    await expect(canvas.getByDisplayValue("Custom Format")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(args.onSaved).toHaveBeenCalled();
  },
};

export const ExternalConverterMode: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const select = canvas.getByLabelText("Conversion Pipeline");
    await userEvent.selectOptions(select, "external");
    await expect(canvas.getByPlaceholderText("e.g. pdftotext")).toBeVisible();
  },
};
