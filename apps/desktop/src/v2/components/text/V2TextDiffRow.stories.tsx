import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import V2TextDiffRow, { type HunkMeta } from "./V2TextDiffRow";

const CHANGED_HUNK: HunkMeta = {
  ordinal: 0,
  role: "single",
  hunk: { ordinal: 0, start: 1, end: 1 },
};

const START_HUNK: HunkMeta = {
  ordinal: 1,
  role: "start",
  hunk: { ordinal: 1, start: 2, end: 4 },
};

const meta = {
  title: "V2/Text/V2TextDiffRow",
  component: V2TextDiffRow,
  tags: ["autodocs"],
  decorators: [
    (Story) => (
      <div className="v2-mono overflow-hidden rounded-lg border border-[var(--v2-border)] bg-[var(--v2-surface)] text-[12px] leading-[21px]">
        <Story />
      </div>
    ),
  ],
  args: {
    row: {
      index: 1,
      kind: "changed",
      left: 'const timeoutMs = 2500; // staging',
      right: 'const timeoutMs = 5000; // production',
      inline: [
        { aStart: 18, aEnd: 22, bStart: 18, bEnd: 22 },
        { aStart: 27, aEnd: 34, bStart: 27, bEnd: 37 },
      ],
    },
    meta: CHANGED_HUNK,
    leftLineNo: 12,
    rightLineNo: 12,
    isSelected: false,
    isUnimportant: false,
    isManualAnchor: false,
    leftSyntaxLang: "code",
    rightSyntaxLang: "code",
    alignPickTargetSide: null,
    activeSide: null,
    activeText: "",
    activeCol: 0,
    focusNonce: 0,
    splitPct: 50,
    dragging: false,
    onSelect: fn(),
    onFocusCell: fn(),
    onCellChange: fn(),
    onCaretMove: fn(),
    onSplitEnter: fn(),
    onBackspaceStart: fn(),
    onDeleteEnd: fn(),
    onArrowVertical: fn(),
    onMultiLinePaste: fn(),
    onBlurCommit: fn(),
    onCopyHunk: fn(),
    onCopyLine: fn(),
    onStartDrag: fn(),
    onResetSplit: fn(),
  },
} satisfies Meta<typeof V2TextDiffRow>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ChangedWithInlineSpans: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId("v2/copy-block-left-1"));
    await expect(args.onCopyLine).toHaveBeenCalledWith(1, "left");
  },
};

export const EqualSyntaxHighlighted: Story = {
  args: {
    meta: undefined,
    row: {
      index: 0,
      kind: "equal",
      left: 'export async function compare(left: string, right: string): Promise<number> {',
      right: 'export async function compare(left: string, right: string): Promise<number> {',
      inline: [],
    },
  },
};

export const OrphanLeftWithHatching: Story = {
  args: {
    meta: START_HUNK,
    rightLineNo: null,
    isManualAnchor: true,
    row: {
      index: 2,
      kind: "leftOnly",
      left: "  legacyFlag: true,",
      right: null,
      inline: [],
    },
  },
};

export const UnimportantMinorBlue: Story = {
  args: {
    isUnimportant: true,
  },
};

export const ActiveInlineEditor: Story = {
  args: {
    isSelected: true,
    activeSide: "left",
    activeText: "const timeoutMs = 2500; // editing inline",
    activeCol: 20,
  },
};
