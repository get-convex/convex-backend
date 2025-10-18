import { Meta, StoryObj } from "@storybook/nextjs";
import { DateRangePicker } from "@common/elements/DateRangePicker";
import { fn } from "storybook/test";

const meta = {
  component: DateRangePicker,
  args: {
    date: {
      from: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000),
      to: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
    },
    setDate: fn(),
  },
} satisfies Meta<typeof DateRangePicker>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const RestrictedRange: Story = {
  args: {
    minDate: new Date(Date.now() - 4 * 7 * 24 * 60 * 60 * 1000),
    maxDate: new Date(Date.now() + 4 * 7 * 24 * 60 * 60 * 1000),
    beforeMinDateTooltip: (
      <>
        This is <em>too early</em>!
      </>
    ),
  },
};
