import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2MenuBar from "./V2MenuBar";

const meta = {
  title: "V2/Chrome/V2MenuBar",
  component: V2MenuBar,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    storyFrameClassName: "min-h-[380px]",
  },
  decorators: [
    (Story) => (
      <div className="min-h-[380px] bg-[var(--v2-bg)]">
        <Story />
      </div>
    ),
  ],
  args: {
    activeTab: {
      id: "tab-text-1",
      kind: "text",
      title: "t_left.txt ↔ t_right.txt",
      leftPath: "tools/parity-harness/fixtures/t_left.txt",
      rightPath: "tools/parity-harness/fixtures/t_right.txt",
    },
    commands: {
      filterMode: "all",
      setFilterMode: fn(),
      contextEnabled: true,
      toggleContext: fn(),
      minorEnabled: false,
      toggleMinor: fn(),
      openRules: fn(),
      openFind: fn(),
      canCopy: true,
      copySelected: fn(),
      copySelectedRight: fn(),
      hasSections: true,
      prevSection: fn(),
      nextSection: fn(),
      swapSides: fn(),
      reload: fn(),
    },
    showToolbar: true,
    layoutDensity: "standard",
    paneOrientation: "side-by-side",
    theme: "light",
    onCommand: fn(),
    onOpenSessionKind: fn(),
  },
} satisfies Meta<typeof V2MenuBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ComparisonContext: Story = {};

export const HomeContext: Story = {
  args: {
    activeTab: {
      id: "home",
      kind: "home",
      title: "Home",
      leftPath: "",
      rightPath: "",
    },
    commands: {},
  },
};

export const ViewMenuOpen: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/menu-view"));
    const overUnder = await canvas.findByRole("button", { name: /Over-Under Layout/i });
    await userEvent.click(overUnder);
    await expect(args.onCommand).toHaveBeenCalledWith("pane_over_under");
  },
};

export const SessionSubmenuOpen: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/menu-session"));
    const newSessionItem = await canvas.findByRole("button", { name: "New Session" });
    await userEvent.hover(newSessionItem);
    await expect(await canvas.findByRole("button", { name: /JSON \/ YAML Diff/i })).toBeVisible();
  },
};
