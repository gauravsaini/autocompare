import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2ErrorBanner from "./V2ErrorBanner";

const meta = {
  title: "V2/Chrome/V2ErrorBanner",
  component: V2ErrorBanner,
  tags: ["autodocs"],
  args: {
    message: "Failed to read /Users/demo/secret.txt: permission denied (os error 13)",
    onDismiss: fn(),
  },
} satisfies Meta<typeof V2ErrorBanner>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Dismissible: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const alert = canvas.getByRole("alert");
    await expect(alert).toHaveTextContent("permission denied");
    await userEvent.click(canvas.getByRole("button", { name: "Dismiss" }));
    await expect(args.onDismiss).toHaveBeenCalledTimes(1);
  },
};

export const LongTruncatedError: Story = {
  args: {
    message:
      "Comparison failed while streaming /Volumes/ExternalDrive/archive/2026-11-28/deep/nested/directory/with/a/very-long-file-name-for-visual-truncation-testing.json: unexpected end of JSON input at line 4182 column 19",
    onDismiss: undefined,
  },
};

export const EmptyHidden: Story = {
  args: {
    message: null,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("alert")).toBeNull();
  },
};
