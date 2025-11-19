import { Meta, StoryObj } from "@storybook/nextjs";
import { DetailPanel } from "@common/elements/DetailPanel";
import { fn } from "storybook/test";

const meta = {
  component: DetailPanel,
  render: (args) => <DetailPanel {...args} />,
  args: {
    onClose: fn(),
  },
} satisfies Meta<typeof DetailPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    header: "Detail Panel Title",
    content: "Hello world!",
  },
};

export const WithError: Story = {
  args: {
    header: "Detail Panel Title",
    content: "Some content",
    error: "An error occurred while loading the details",
  },
};

export const Loading: Story = {
  args: {
    header: "Detail Panel Title",
    content: undefined,
  },
};
