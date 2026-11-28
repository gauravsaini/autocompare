import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2ReportExportModal from "./V2ReportExportModal";

const meta = {
  title: "V2/Modals/V2ReportExportModal",
  component: V2ReportExportModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    tab: {
      id: "text-1",
      kind: "text",
      title: "t_left.txt ↔ t_right.txt",
      leftPath: "tools/parity-harness/fixtures/t_left.txt",
      rightPath: "tools/parity-harness/fixtures/t_right.txt",
    },
    onClose: fn(),
  },
} satisfies Meta<typeof V2ReportExportModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const HtmlOptions: Story = {};

export const PatchWithPreview: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/report-format-patch"));
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    await expect(await canvas.findByTestId("v2/report-preview")).toHaveTextContent("--- a/");
  },
};

export const ExportSuccess: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/report-export-btn"));
    await expect(await canvas.findByTestId("v2/report-result")).toHaveTextContent("Exported HTML report");
  },
};
