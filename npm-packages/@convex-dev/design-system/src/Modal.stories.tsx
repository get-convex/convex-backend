import { Meta, StoryObj } from "@storybook/nextjs";
import { Modal } from "@ui/Modal";
import { fn } from "storybook/test";

const meta = {
  component: Modal,
  render: (args) => (
    <Modal {...args}>
      Modal content... maybe there's even a button in here 😮
    </Modal>
  ),
  args: {
    title: "Modal title",
    description: "Detailed description of the modal's purpose",
    onClose: fn(),
    children: <p>Hello world!</p>,
  },
} satisfies Meta<typeof Modal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const Large: Story = {
  args: {
    size: "lg",
  },
};
