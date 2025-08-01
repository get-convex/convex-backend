import { Meta, StoryObj } from "@storybook/nextjs";
import { TimestampDistance } from "@common/elements/TimestampDistance";

const meta = {
  component: TimestampDistance,
  render: (args) => (
    <div className="m-auto w-fit">
      <TimestampDistance {...args}>TimestampDistance content</TimestampDistance>
    </div>
  ),
} satisfies Meta<typeof TimestampDistance>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    date: new Date("12/19/2022, 10:00:00 AM"),
  },
};
