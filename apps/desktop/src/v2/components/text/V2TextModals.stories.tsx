import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { V2TextRulesModal, type TextSessionRules } from "./V2TextModals";

const DEFAULT_RULES: TextSessionRules = {
  leftFormat: "Source Code",
  rightFormat: "Source Code",
  sameFormat: true,
  leftEncoding: "UTF-8",
  rightEncoding: "UTF-8",
  sameEncoding: true,
  eol: "UNIX",
  ignoreWs: false,
  ignoreCase: false,
  ignoreBlank: false,
  minorEnabled: false,
  unimportantRules: [],
  alignmentMode: "standard",
  neverAlignDifferences: false,
  skewTolerance: 2000,
  useClosenessMatching: true,
  replacements: [],
};

const meta = {
  title: "V2/Modals/V2TextRulesModal",
  component: V2TextRulesModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    initialTab: "importance",
    rules: DEFAULT_RULES,
    leftPath: "tools/parity-harness/fixtures/t_left.txt",
    rightPath: "tools/parity-harness/fixtures/t_right.txt",
    onClose: fn(),
    onApply: fn(),
  },
} satisfies Meta<typeof V2TextRulesModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ImportanceTabWithPresets: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/rules-add-preset-0"));
    await userEvent.click(canvas.getByTestId("v2/rules-ok"));
    await expect(args.onApply).toHaveBeenCalledTimes(1);
  },
};

export const SpecsTab: Story = {
  args: { initialTab: "specs" },
};

export const FormatTab: Story = {
  args: { initialTab: "format" },
};

export const AlignmentTab: Story = {
  args: { initialTab: "alignment" },
};

export const ReplacementsTab: Story = {
  args: { initialTab: "replacements" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/rules-add-replacement"));
    await expect(canvas.getByPlaceholderText("Left text or regex…")).toBeVisible();
  },
};
