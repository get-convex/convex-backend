import { Meta, StoryObj } from "@storybook/nextjs";
import { BackupRestoreOngoing } from "./BackupRestoreStatus";

export default {
  component: BackupRestoreOngoing,
} as Meta<typeof BackupRestoreOngoing>;

type Story = StoryObj<typeof BackupRestoreOngoing>;

export const Primary: Story = {
  args: {
    progressMessage: 'Importing "_storage"',
  },
};
