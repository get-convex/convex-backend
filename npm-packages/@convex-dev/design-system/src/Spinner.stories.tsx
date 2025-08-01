import { Meta, StoryObj } from "@storybook/nextjs";
import { Spinner } from "@ui/Spinner";

const meta = {
  component: Spinner,
  render: (args: any) => <Spinner {...args} />,
} satisfies Meta<typeof Spinner>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};
