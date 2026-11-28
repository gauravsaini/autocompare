import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2ReloadPromptBanner from "./V2ReloadPromptBanner";

const meta = {
  title: "V2/Chrome/V2ReloadPromptBanner",
  component: V2ReloadPromptBanner,
  tags: ["autodocs"],
  args: {
    changedPaths: ["/workspace/src/main.rs"],
    onReload: fn(),
    onDismiss: fn(),
  },
} satisfies Meta<typeof V2ReloadPromptBanner>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SingleFileChanged: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("status")).toHaveTextContent("main.rs changed on disk.");
    await userEvent.click(canvas.getByTestId("v2/reload-banner-confirm"));
    await expect(args.onReload).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByTestId("v2/reload-banner-dismiss"));
    await expect(args.onDismiss).toHaveBeenCalledTimes(1);
  },
};

export const BothSidesChanged: Story = {
  args: {
    changedPaths: ["/workspace/left/config.yaml", "/workspace/right/config.yaml"],
  },
};
