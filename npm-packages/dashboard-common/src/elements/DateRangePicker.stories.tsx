import { Meta, StoryObj } from "@storybook/nextjs";
import { DateRangePicker } from "@common/elements/DateRangePicker";
import { fn } from "storybook/test";

export const Primary: Story = {
  args: {
    date: {
      from: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000),
      to: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
    },
    setDate: fn(),
  },
};

const meta = { component: DateRangePicker } satisfies Meta<
  typeof DateRangePicker
>;

export default meta;
type Story = StoryObj<typeof meta>;
