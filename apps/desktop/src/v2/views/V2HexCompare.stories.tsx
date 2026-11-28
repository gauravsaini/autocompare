import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2HexCompare from "./V2HexCompare";

const meta = {
  title: "V2/Views/V2HexCompare",
  component: V2HexCompare,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    leftPath: "",
    rightPath: "",
    paneOrientation: "side-by-side",
    onPathsChange: fn(),
    onRegisterCommands: fn(),
    onStatusChange: fn(),
  },
} satisfies Meta<typeof V2HexCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SideBySideWithInspector: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const firstByte = (await canvas.findAllByTitle(/click to inspect/i))[0];
    await userEvent.click(firstByte);
    await expect(await canvas.findByTestId("v2/hex-inspector")).toBeVisible();
  },
};

export const OverUnderLayout: Story = {
  args: {
    paneOrientation: "over-under",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/Top Pane — Left Hex/i)).toBeVisible();
  },
};
