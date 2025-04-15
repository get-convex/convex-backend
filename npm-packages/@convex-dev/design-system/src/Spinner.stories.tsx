import { Meta, StoryObj } from "@storybook/react";
import { Spinner } from "@ui/Spinner";

export default {
  component: Spinner,
  render: (args: any) => <Spinner {...args} />,
} as Meta<typeof Spinner>;

export const Default: StoryObj<typeof Spinner> = {
  args: {},
};
