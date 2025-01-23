import { Meta, StoryObj } from "@storybook/react";
import { DeploymentResponse, Team } from "generatedApi";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { BackupResponse } from "api/backups";
import { BackupRestoreSuccess } from "./BackupRestoreStatus";

const oneDayAgo = new Date();
oneDayAgo.setDate(oneDayAgo.getDate() - 1);

const inOneWeek = new Date();
inOneWeek.setDate(inOneWeek.getDate() + 7);

const team: Team = {
  id: 1,
  creator: 1,
  slug: "team",
  name: "Team",
  suspended: false,
};

const backup: BackupResponse = {
  id: 1,
  snapshotId: "yo" as Id<"_exports">,
  sourceDeploymentId: 1,
  sourceDeploymentName: "joyful-capybara-123",
  state: "complete",
  requestedTime: +oneDayAgo,
  expirationTime: +inOneWeek,
};

const deployment: DeploymentResponse = {
  kind: "cloud",
  id: 1,
  name: "joyful-capybara-123",
  deploymentType: "prod",
  createTime: +oneDayAgo,
  projectId: 1,
  creator: 1,
  previewIdentifier: null,
};

export default {
  component: BackupRestoreSuccess,
  args: {
    completedTime: new Date(),
    restoredRowsCount: 336_185,
    deployment,
    team,
    backup,
    snapshotImportCheckpoints: undefined,
  },
} as Meta<typeof BackupRestoreSuccess>;

type Story = StoryObj<typeof BackupRestoreSuccess>;

export const Primary: Story = {
  args: {
    completedTime: new Date(),
    restoredRowsCount: 336_185,
  },
};
