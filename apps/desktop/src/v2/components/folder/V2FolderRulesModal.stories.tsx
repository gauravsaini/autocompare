import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2FolderRulesModal from "./V2FolderRulesModal";

const meta = {
  title: "V2/Modals/V2FolderRulesModal",
  component: V2FolderRulesModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    initialTab: "comparison",
    rules: {
      contentCompare: false,
      timestampToleranceSec: 2,
      masks: "-node_modules/*; -target/*",
    },
    onClose: fn(),
    onApply: fn(),
  },
} satisfies Meta<typeof V2FolderRulesModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ComparisonTab: Story = {};

export const NameFiltersTab: Story = {
  args: {
    initialTab: "filters",
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /\+ -\.git/i }));
    await expect(canvas.getByTestId("v2/folder-masks-input")).toHaveValue(
      "-node_modules/*; -target/*; -.git/*",
    );
    await userEvent.click(canvas.getByTestId("v2/folder-rules-ok"));
    await expect(args.onApply).toHaveBeenCalledTimes(1);
  },
};
