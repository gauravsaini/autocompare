import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2TableRulesModal from "./V2TableRulesModal";

const meta = {
  title: "V2/Modals/V2TableRulesModal",
  component: V2TableRulesModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    initialTab: "columns",
    columnNames: ["id", "service", "region", "latency_ms", "status"],
    rules: {
      byKey: true,
      keyColumns: [0, 1],
      delimiter: ",",
      ignoreCase: false,
      ignoreSpace: true,
      numericTolerance: "0.01",
    },
    onClose: fn(),
    onApply: fn(),
  },
} satisfies Meta<typeof V2TableRulesModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ColumnsTab: Story = {};

export const FormatTab: Story = {
  args: { initialTab: "format" },
};

export const ImportanceTab: Story = {
  args: { initialTab: "importance" },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "±0.001" }));
    await userEvent.click(canvas.getByTestId("v2/table-rules-ok"));
    await expect(args.onApply).toHaveBeenCalledTimes(1);
  },
};
