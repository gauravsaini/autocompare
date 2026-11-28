import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2MediaCompare from "./V2MediaCompare";

const meta = {
  title: "V2/Views/V2MediaCompare",
  component: V2MediaCompare,
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
} satisfies Meta<typeof V2MediaCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PictureSliderWipe: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/media-stage", {}, { timeout: 5000 }),
    ).toBeVisible();
    await expect(canvas.getByText("Visual Compare — Slider Wipe")).toBeVisible();
  },
};

export const SideBySideMode: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/media-stage", {}, { timeout: 5000 }),
    ).toBeVisible();
    await userEvent.click(canvas.getByTestId("v2/media-mode-sideBySide"));
    await expect(canvas.getByText("Visual Compare — Side-by-Side")).toBeVisible();
  },
};

export const OnionSkinMode: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/media-stage", {}, { timeout: 5000 }),
    ).toBeVisible();
    await userEvent.click(canvas.getByTestId("v2/media-mode-onion"));
    await expect(canvas.getByText("Visual Compare — Onion Skin")).toBeVisible();
  },
};

export const PixelDiffMode: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/media-stage", {}, { timeout: 5000 }),
    ).toBeVisible();
    await userEvent.click(canvas.getByTestId("v2/media-mode-diff"));
    await expect(canvas.getByText("Visual Compare — Pixel Diff")).toBeVisible();
  },
};

export const AudioCompareMode: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByTestId("v2/media-stage", {}, { timeout: 5000 }),
    ).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Audio" }));
    await expect(
      await canvas.findByText("Audio Stream Similarity", {}, { timeout: 5000 }),
    ).toBeVisible();
  },
};
