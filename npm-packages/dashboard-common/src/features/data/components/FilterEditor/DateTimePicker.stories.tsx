import { Meta, StoryObj } from "@storybook/nextjs";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";

export const Primary: Story = {
  args: {
    date: new Date(),
    onChange: () => {},
  },
};

const meta = { component: DateTimePicker } satisfies Meta<
  typeof DateTimePicker
>;

export default meta;
type Story = StoryObj<typeof meta>;
