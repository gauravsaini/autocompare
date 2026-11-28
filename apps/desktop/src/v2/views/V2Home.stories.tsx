import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { seedStorySessions } from "../stories/mockSessions";
import V2Home from "./V2Home";

const meta = {
  title: "V2/Views/V2Home",
  component: V2Home,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
  },
  args: {
    demoMode: false,
    onNewSession: fn(),
    onOpenSessionManager: fn(),
    onOpenNewSessionModal: fn(),
    onOpenCommandPalette: fn(),
    openComparisonsCount: 0,
    refreshSignal: 0,
  },
} satisfies Meta<typeof V2Home>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultEmptySessions: Story = {
  loaders: [
    async () => {
      seedStorySessions({ empty: true, sidebarOpen: true, sidebarWidth: 268 });
      return {};
    },
  ],
};

export const PopulatedWithDemoMode: Story = {
  args: {
    demoMode: true,
    openComparisonsCount: 2,
    refreshSignal: 1,
  },
  loaders: [
    async () => {
      seedStorySessions({ empty: false, sidebarOpen: true, sidebarWidth: 268 });
      return {};
    },
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("API Config JSON Diff")).toBeVisible();
    await expect(canvas.getByTestId("v2/demo-text-btn")).toBeVisible();
  },
};

export const SidebarCollapsed: Story = {
  loaders: [
    async () => {
      seedStorySessions({ empty: false, sidebarOpen: false });
      return {};
    },
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByTestId("v2/sidebar-dock")).toBeVisible();
  },
};

export const SpotlightPathLaunch: Story = {
  loaders: [
    async () => {
      seedStorySessions({ empty: false, sidebarOpen: true, sidebarWidth: 268 });
      return {};
    },
  ],
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const input = canvas.getByTestId("v2/spotlight-input");
    await userEvent.type(input, "/tmp/left.json /tmp/right.json");
    const launchBanner = await canvas.findByTestId("v2/spotlight-paths-action");
    await expect(launchBanner).toHaveTextContent("JSON / YAML Diff");
    await userEvent.click(launchBanner);
    await expect(args.onNewSession).toHaveBeenCalledWith(
      "structured",
      "/tmp/left.json",
      "/tmp/right.json",
    );
  },
};
