import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2TextCompare from "./V2TextCompare";

const meta = {
  title: "V2/Views/V2TextCompare",
  component: V2TextCompare,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    leftPath: "",
    rightPath: "",
    paneOrientation: "side-by-side",
    onPathsChange: fn(),
    onDirtyChange: fn(),
    onRegisterCommands: fn(),
    onStatusChange: fn(),
  },
} satisfies Meta<typeof V2TextCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SideBySideDemoDiff: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/text-row-0")).toBeVisible();
    await expect(canvas.queryByTestId("v2/error-banner")).toBeNull();
  },
};

export const OverUnderLayout: Story = {
  args: {
    paneOrientation: "over-under",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/Top Pane \(Left\)/i)).toBeVisible();
  },
};

export const FindAndReplaceOpen: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/text-row-0")).toBeVisible();
    await userEvent.keyboard("{Control>}h{/Control}");
    const findInput = await canvas.findByPlaceholderText("Find in diff…");
    await userEvent.type(findInput, "common");
    await expect(canvas.getByTestId("v2/replace-input")).toBeVisible();
  },
};
