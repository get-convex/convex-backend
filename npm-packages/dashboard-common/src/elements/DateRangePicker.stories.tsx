import { StoryObj } from "@storybook/react";
import { DateRangePicker } from "@common/elements/DateRangePicker";

export const Primary: StoryObj<typeof DateRangePicker> = {
  args: {
    date: {
      from: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000),
      to: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
    },
    setDate: () => {},
  },
};

export default { component: DateRangePicker };
