import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import V2App from "./V2App";
import { seedStorySessions } from "./stories/mockSessions";

const meta = {
  title: "V2/Shell/V2App",
  component: V2App,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta<typeof V2App>;

export default meta;
type Story = StoryObj<typeof meta>;

export const HomeLauncher: Story = {
  loaders: [
    async () => {
      window.localStorage.clear();
      seedStorySessions({ empty: false, sidebarOpen: true, sidebarWidth: 268 });
      return {};
    },
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("v2/home")).toBeVisible();
    await expect(canvas.getByTestId("v2/menubar")).toBeVisible();
    await expect(canvas.getByTestId("v2/statusbar")).toBeVisible();
  },
};

export const ActiveTextCompareSession: Story = {
  loaders: [
    async () => {
      window.localStorage.clear();
      seedStorySessions({ empty: false, sidebarOpen: true, sidebarWidth: 268 });
      return {};
    },
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByTestId("v2/home-card-text"));
    await expect(await canvas.findByTestId("v2/text-view")).toBeVisible();
    await expect(canvas.getByTestId("v2/toolbar")).toBeVisible();
  },
};
