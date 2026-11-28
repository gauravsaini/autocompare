import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { tomlXmlPlugin } from "./tomlXmlPlugin";

const V2TomlXmlCompare = tomlXmlPlugin.component;

const meta = {
  title: "V2/Plugins/V2TomlXmlCompare",
  component: V2TomlXmlCompare,
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
} satisfies Meta<typeof V2TomlXmlCompare>;

export default meta;
type Story = StoryObj<typeof meta>;

export const TomlConfigDiff: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByTestId("v2/toml-xml-view")).toBeVisible();
    await expect(canvas.getByText("package.version")).toBeVisible();
  },
};

export const XmlModeAndDiffsOnly: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const select = canvas.getByLabelText("Config format");
    await userEvent.selectOptions(select, "xml");
    const leftArea = canvas.getByLabelText("Left config");
    const rightArea = canvas.getByLabelText("Right config");
    await userEvent.clear(leftArea);
    await userEvent.type(leftArea, "<config><port>3000</port><mode>dev</mode></config>");
    await userEvent.clear(rightArea);
    await userEvent.type(rightArea, "<config><port>8080</port><mode>dev</mode></config>");
    await userEvent.click(canvas.getByLabelText("Only differences"));
    await expect(canvas.getByText("config.port")).toBeVisible();
  },
};
