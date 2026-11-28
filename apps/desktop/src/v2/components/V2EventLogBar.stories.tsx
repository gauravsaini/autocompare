import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import type { UiEventLogState } from "../utils/sessionEventLog";
import V2EventLogBar from "./V2EventLogBar";

const FIXED_TS = Date.UTC(2026, 10, 28, 3, 15, 0);

const EMPTY_LOG: UiEventLogState<string> = {
  events: [],
  cursor: 0,
};

const POPULATED_LOG: UiEventLogState<string> = {
  events: [
    { seq: 1, timestampMs: FIXED_TS, action: "merge_resolve", label: "Conflict #1 → left", payload: "left" },
    { seq: 2, timestampMs: FIXED_TS + 5000, action: "merge_resolve", label: "Conflict #2 → right", payload: "right" },
    { seq: 3, timestampMs: FIXED_TS + 10000, action: "merge_resolve", label: "Conflict #3 → custom", payload: "custom" },
  ],
  cursor: 2,
};

const meta = {
  title: "V2/Chrome/V2EventLogBar",
  component: V2EventLogBar<string>,
  tags: ["autodocs"],
  args: {
    log: POPULATED_LOG,
    onUndo: fn(),
    onRedo: fn(),
    onCopied: fn(),
  },
} satisfies Meta<typeof V2EventLogBar<string>>;

export default meta;
type Story = StoryObj<typeof meta>;

export const EmptyLog: Story = {
  args: {
    log: EMPTY_LOG,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: /Undo/i })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: /Redo/i })).toBeDisabled();
  },
};

export const WithUndoRedo: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /Undo/i }));
    await expect(args.onUndo).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: /Redo/i }));
    await expect(args.onRedo).toHaveBeenCalledTimes(1);
  },
};

export const ExpandedAuditDrawer: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /Audit Log/i }));
    await expect(canvas.getByText(/Conflict #3 → custom/)).toBeVisible();
  },
};
