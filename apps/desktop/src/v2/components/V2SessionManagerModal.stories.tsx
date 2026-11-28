import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { seedStorySessions } from "../stories/mockSessions";
import V2SessionManagerModal from "./V2SessionManagerModal";

const meta = {
  title: "V2/Modals/V2SessionManagerModal",
  component: V2SessionManagerModal,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    open: true,
    onClose: fn(),
    onOpenSession: fn(),
    onSessionsChanged: fn(),
  },
} satisfies Meta<typeof V2SessionManagerModal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PopulatedSessions: Story = {
  loaders: [
    async () => {
      seedStorySessions();
      return {};
    },
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByText("API Config JSON Diff", {}, { timeout: 5000 }),
    ).toBeVisible();
    await userEvent.type(canvas.getByPlaceholderText("Search saved sessions…"), "Release");
    await expect(canvas.getByText("Release Build Folder Check")).toBeVisible();
  },
};

export const EmptyState: Story = {
  loaders: [
    async () => {
      seedStorySessions({ empty: true });
      return {};
    },
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByText("No saved sessions found.", {}, { timeout: 5000 }),
    ).toBeVisible();
  },
};
