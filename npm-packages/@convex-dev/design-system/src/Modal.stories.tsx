import { Meta, StoryObj } from "@storybook/nextjs";
import { Modal } from "@ui/Modal";

const meta = {
  component: Modal,
  render: (args) => (
    <Modal {...args}>
      Modal content... maybe there's even a button in here 😮
    </Modal>
  ),
} satisfies Meta<typeof Modal>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    title: "Modal title",
    description: "Detailed description of the modal's purpose",
  },
};

export const Large: Story = {
  args: {
    title: "Modal title",
    description: "Detailed description of the modal's purpose",
    size: "lg",
  },
};
