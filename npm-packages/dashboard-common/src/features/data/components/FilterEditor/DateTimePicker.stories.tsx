import { Meta, StoryObj } from "@storybook/nextjs";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";
import { fn } from "storybook/test";

const meta = {
  component: DateTimePicker,
  args: {
    onChange: fn(),
    date: new Date(),
  },
} satisfies Meta<typeof DateTimePicker>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const RestrictedRange: Story = {
  args: {
    minDate: new Date(Date.now() - 4 * 7 * 24 * 60 * 60 * 1000),
    maxDate: new Date(Date.now() + 4 * 7 * 24 * 60 * 60 * 1000),
    onChange: fn(),
  },
};
