import { StoryObj } from "@storybook/react";
import { DateTimePicker } from "./DateTimePicker";

export const Primary: StoryObj<typeof DateTimePicker> = {
  args: {
    date: new Date(),
    onChange: () => {},
  },
};

export default { component: DateTimePicker };
