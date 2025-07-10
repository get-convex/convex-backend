import { Meta, StoryObj } from "@storybook/nextjs";
import { Modal } from "@ui/Modal";

export default {
  component: Modal,
  render: (args) => (
    <Modal {...args}>
      Modal content... maybe there's even a button in here 😮
    </Modal>
  ),
} as Meta<typeof Modal>;

export const Primary: StoryObj<typeof Modal> = {
  args: {
    title: "Modal title",
    description: "Detailed description of the modal's purpose",
  },
};

export const Large: StoryObj<typeof Modal> = {
  args: {
    title: "Modal title",
    description: "Detailed description of the modal's purpose",
    size: "lg",
  },
};
