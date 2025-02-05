import { Meta, StoryObj } from "@storybook/react";
import { TimestampDistance } from "@common/elements/TimestampDistance";

export default {
  component: TimestampDistance,
  render: (args) => (
    <div className="m-auto w-fit">
      <TimestampDistance {...args}>TimestampDistance content</TimestampDistance>
    </div>
  ),
} as Meta<typeof TimestampDistance>;

export const Primary: StoryObj<typeof TimestampDistance> = {
  args: {
    date: new Date("12/19/2022, 10:00:00 AM"),
  },
};
