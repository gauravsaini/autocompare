import type { Meta, StoryObj } from "@storybook/react-vite";
import { useRef } from "react";
import { expect, fn, userEvent, within } from "storybook/test";
import V2TextFindBar, { type V2TextFindBarProps } from "./V2TextFindBar";

function FindBarHarness(
  props: Omit<V2TextFindBarProps, "inputRef" | "replaceInputRef">,
) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const replaceInputRef = useRef<HTMLInputElement | null>(null);
  return <V2TextFindBar {...props} inputRef={inputRef} replaceInputRef={replaceInputRef} />;
}

const meta = {
  title: "V2/Text/V2TextFindBar",
  component: FindBarHarness,
  tags: ["autodocs"],
  args: {
    query: "latency",
    replaceQuery: "duration",
    replaceOpen: false,
    matchCase: false,
    targetSide: "left",
    hitCount: 4,
    activeHit: 1,
    onQueryChange: fn(),
    onReplaceQueryChange: fn(),
    onToggleReplaceOpen: fn(),
    onToggleMatchCase: fn(),
    onTargetSideChange: fn(),
    onStep: fn(),
    onReplaceOne: fn(),
    onReplaceAll: fn(),
    onClose: fn(),
  },
} satisfies Meta<typeof V2TextFindBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const FindOnly: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("2 of 4")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Next" }));
    await expect(args.onStep).toHaveBeenCalledWith(1);
  },
};

export const ReplaceExpanded: Story = {
  args: {
    replaceOpen: true,
    matchCase: true,
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/replace-all"));
    await expect(args.onReplaceAll).toHaveBeenCalledTimes(1);
  },
};

export const NoMatches: Story = {
  args: {
    query: "missingToken",
    hitCount: 0,
    activeHit: 0,
  },
};
