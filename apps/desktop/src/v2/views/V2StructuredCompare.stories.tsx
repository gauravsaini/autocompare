import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2StructuredCompare from "./V2StructuredCompare";

const meta = {
  title: "V2/Views/V2StructuredCompare",
  component: V2StructuredCompare,
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
} satisfies Meta<typeof V2StructuredCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const JsonAstDiff: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByTestId("v2/structured-view")).toHaveAttribute(
      "data-structured-status",
      "ok",
    );
    await expect(canvas.getByText("$.database.host")).toBeVisible();
  },
};

export const IgnoreArrayOrderAndSorted: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByLabelText("Ignore array order"));
    await userEvent.click(canvas.getByRole("button", { name: /Sort Keys & Format/i }));
    await expect(canvas.getByTestId("v2/structured-view")).toHaveAttribute(
      "data-canonicalized",
      "true",
    );
  },
};

export const ParseErrorBanner: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const leftEditor = canvasElement.querySelectorAll("textarea")[0];
    await userEvent.clear(leftEditor);
    await userEvent.type(leftEditor, "{{broken");
    await expect(await canvas.findByTestId("v2/error-banner")).toBeVisible();
  },
};
