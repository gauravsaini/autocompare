import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2Toolbar from "./V2Toolbar";

const meta = {
  title: "V2/Chrome/V2Toolbar",
  component: V2Toolbar,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
  },
  args: {
    onGoHome: fn(),
    onOpenSessions: fn(),
    commands: {
      filterMode: "diffs",
      setFilterMode: fn(),
      contextEnabled: true,
      toggleContext: fn(),
      minorEnabled: false,
      toggleMinor: fn(),
      openRules: fn(),
      openFormat: fn(),
      openFind: fn(),
      canCopy: true,
      copySelected: fn(),
      hasSections: true,
      prevSection: fn(),
      nextSection: fn(),
      swapSides: fn(),
      reload: fn(),
    },
  },
} satisfies Meta<typeof V2Toolbar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const TextCompareActive: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/toolbar-all"));
    await expect(args.commands.setFilterMode).toHaveBeenCalledWith("all");
    await userEvent.click(canvas.getByTestId("v2/toolbar-next"));
    await expect(args.commands.nextSection).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByTestId("v2/toolbar-home"));
    await expect(args.onGoHome).toHaveBeenCalledTimes(1);
  },
};

export const MinorAndSameActive: Story = {
  args: {
    commands: {
      filterMode: "same",
      setFilterMode: fn(),
      contextEnabled: false,
      toggleContext: fn(),
      minorEnabled: true,
      toggleMinor: fn(),
      openRules: fn(),
      openFormat: fn(),
      openFind: fn(),
      canCopy: false,
      hasSections: false,
      swapSides: fn(),
      reload: fn(),
    },
  },
};

export const ReadOnlyPluginSubset: Story = {
  args: {
    commands: {},
  },
};
