import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2TableCompare from "./V2TableCompare";

const meta = {
  title: "V2/Views/V2TableCompare",
  component: V2TableCompare,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    leftPath: "",
    rightPath: "",
    onPathsChange: fn(),
    onRegisterCommands: fn(),
    onStatusChange: fn(),
  },
} satisfies Meta<typeof V2TableCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultCsvGrid: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/table-view")).toBeVisible();
    await expect(canvas.queryByTestId("v2/error-banner")).toBeNull();
  },
};

export const FindBarOpen: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/table-view")).toBeVisible();
    await userEvent.keyboard("{Control>}f{/Control}");
    await expect(await canvas.findByPlaceholderText("Find cell value…")).toBeVisible();
  },
};
