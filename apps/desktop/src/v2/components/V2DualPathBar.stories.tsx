import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, within } from "storybook/test";
import V2DualPathBar from "./V2DualPathBar";

const meta = {
  title: "V2/Chrome/V2DualPathBar",
  component: V2DualPathBar,
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
  },
  args: {
    leftValue: "tools/parity-harness/fixtures/t_left.txt",
    rightValue: "tools/parity-harness/fixtures/t_right.txt",
    onLeftChange: fn(),
    onRightChange: fn(),
    onSubmit: fn(),
  },
} satisfies Meta<typeof V2DualPathBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithMetadataAndSave: Story = {
  render: (args) => {
    const [left, setLeft] = useState(args.leftValue);
    const [right, setRight] = useState(args.rightValue);
    return (
      <V2DualPathBar
        {...args}
        leftValue={left}
        rightValue={right}
        onLeftChange={(v) => {
          setLeft(v);
          args.onLeftChange(v);
        }}
        onRightChange={(v) => {
          setRight(v);
          args.onRightChange(v);
        }}
        onSaveSide={fn()}
        leftMeta={
          <>
            <span>Nov 28, 10:15 AM</span>
            <span>24.6 KB</span>
            <span>640 lines</span>
            <span>UTF-8</span>
          </>
        }
        rightMeta={
          <>
            <span>Nov 28, 10:18 AM</span>
            <span>24.9 KB</span>
            <span>642 lines</span>
            <span>UTF-8</span>
          </>
        }
      />
    );
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const leftInput = canvas.getByTestId("v2/path-left");
    await userEvent.clear(leftInput);
    await userEvent.type(leftInput, "/tmp/a.txt{enter}");
    await expect(args.onSubmit).toHaveBeenCalledWith("/tmp/a.txt", args.rightValue);
  },
};

export const EmptyPlaceholders: Story = {
  args: {
    leftValue: "",
    rightValue: "",
    leftPlaceholder: "Left folder path…",
    rightPlaceholder: "Right folder path…",
    directory: true,
  },
};

export const AsymmetricSplit: Story = {
  args: {
    splitPct: 35,
    onSaveSide: fn(),
  },
};
