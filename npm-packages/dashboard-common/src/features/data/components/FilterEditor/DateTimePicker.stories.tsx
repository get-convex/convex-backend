import { Meta, StoryObj } from "@storybook/nextjs";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";
import { fn } from "storybook/test";

export const Primary: Story = {
  args: {
    date: new Date(),
    onChange: fn(),
  },
};

const meta = { component: DateTimePicker } satisfies Meta<
  typeof DateTimePicker
>;

export default meta;
type Story = StoryObj<typeof meta>;
