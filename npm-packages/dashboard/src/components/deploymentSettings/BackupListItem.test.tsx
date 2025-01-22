import { cleanup, render } from "@testing-library/react";
import {
  DeploymentResponse,
  ProjectDetails,
  Team,
  TeamMemberResponse,
} from "generatedApi";
import userEvent from "@testing-library/user-event";
import { BackupResponse, useRestoreFromCloudBackup } from "api/backups";
import { Doc, Id } from "system-udfs/convex/_generated/dataModel";
import { BackupListItem, progressMessageForBackup } from "./BackupListItem";

const now = new Date();

const inOneWeek = new Date();
inOneWeek.setDate(inOneWeek.getDate() + 7);

const oneWeekAgo = new Date();
oneWeekAgo.setDate(oneWeekAgo.getDate() - 7);

const backup: BackupResponse = {
  id: 1,
  snapshotId: "yo" as Id<"_exports">,
  sourceDeploymentId: 1,
  sourceDeploymentName: "joyful-capybara-123",
  state: "complete",
  requestedTime: +now,
  expirationTime: +inOneWeek,
};

const backupInProgress: BackupResponse = { ...backup, state: "inProgress" };

const existingCloudBackupRequested: Doc<"_exports"> = {
  _id: "yo" as Id<"_exports">,
  _creationTime: 2,
  state: "requested",
  requestor: "cloudBackup",
};

const existingCloudBackupInProgress: Doc<"_exports"> = {
  _id: "yo" as Id<"_exports">,
  _creationTime: 2,
  state: "in_progress",
  start_ts: BigInt(3),
  requestor: "cloudBackup",
  progress_message: "progressmsg",
};

const targetDeployment: DeploymentResponse = {
  kind: "cloud",
  id: 1,
  name: "joyful-capybara-123",
  deploymentType: "prod",
  createTime: Date.now(),
  projectId: 1,
  creator: 1,
  previewIdentifier: null,
};

const team: Team = {
  id: 1,
  creator: 1,
  slug: "team",
  name: "Team",
  suspended: false,
};

jest.mock("api/deployments", () => {
  const deployment: DeploymentResponse = {
    kind: "cloud",
    id: 1,
    name: "joyful-capybara-123",
    deploymentType: "prod",
    createTime: +Date.now(),
    projectId: 1,
    creator: 1,
    previewIdentifier: null,
  };

  return {
    useDeploymentById: jest.fn().mockReturnValue(deployment),
    useDeployments: jest.fn().mockReturnValue([deployment]),
  };
});

jest.mock("api/projects", () => {
  const project: ProjectDetails = {
    id: 1,
    teamId: 1,
    slug: "my-project",
    name: "My Project",
    isDemo: false,
    createTime: 0,
  };

  return {
    useCurrentProject: jest.fn().mockReturnValue(project),
    useProjects: jest.fn().mockReturnValue([project]),
    useProjectById: jest.fn().mockReturnValue(project),
  };
});

jest.mock("api/teams", () => {
  const t: Team = {
    id: 1,
    creator: 1,
    slug: "team",
    name: "Team",
    suspended: false,
  };
  const profile: TeamMemberResponse = {
    id: 1,
    email: "nicolas@convex.dev",
    name: "Nicolas Ettlin",
    role: "admin",
  };

  return {
    useCurrentTeam: jest.fn().mockReturnValue(t),
    useTeamMembers: jest.fn().mockReturnValue([profile]),
  };
});

jest.mock("api/backups", () => ({
  useRequestCloudBackup: jest.fn().mockReturnValue(jest.fn()),
  useRestoreFromCloudBackup: jest.fn().mockReturnValue(jest.fn()),
  useListCloudBackups: jest.fn().mockReturnValue([]),
}));

jest.mock("api/profile", () => {
  const profile: TeamMemberResponse = {
    id: 1,
    email: "nicolas@convex.dev",
    name: "Nicolas Ettlin",
    role: "admin",
  };
  return {
    useProfile: jest.fn().mockReturnValue(profile),
  };
});

jest.mock("hooks/deploymentApi.tsx", () => ({
  useGetZipExport: jest.fn().mockReturnValue(jest.fn()),
}));

describe("BackupListItem", () => {
  afterEach(cleanup);

  const getZipExportUrl = jest.fn().mockReturnValue("http://example.com");

  it("allows direct backup restoration in development", async () => {
    const user = userEvent.setup();
    const { getByText, getByTitle, findByText } = render(
      <BackupListItem
        backup={backup}
        restoring={false}
        someBackupInProgress={false}
        someRestoreInProgress={false}
        latestBackupInTargetDeployment={null}
        targetDeployment={{ ...targetDeployment, deploymentType: "dev" }}
        team={team}
        getZipExportUrl={getZipExportUrl}
        canPerformActions
        maxCloudBackups={2}
        progressMessage={null}
      />,
    );

    await user.click(getByTitle("Backup options"));
    await user.click(getByText("Restore"));

    expect(await findByText("Backup before restoring?")).toBeInTheDocument();
    await user.click(getByText("Continue"));

    expect(await findByText("Restore from a backup")).toBeInTheDocument();
    expect(getByText("My Project")).toBeInTheDocument();
    await user.click(getByText("Restore"));

    expect(useRestoreFromCloudBackup()).toHaveBeenCalledWith({ id: 1 });
  });

  it("requires checkbox confirmation when restoring from production", async () => {
    const user = userEvent.setup();
    const { getByText, getByTitle, getByRole, findByText } = render(
      <BackupListItem
        backup={backup}
        restoring={false}
        someBackupInProgress={false}
        someRestoreInProgress={false}
        latestBackupInTargetDeployment={null}
        targetDeployment={targetDeployment}
        team={team}
        getZipExportUrl={getZipExportUrl}
        canPerformActions
        maxCloudBackups={2}
        progressMessage={null}
      />,
    );

    await user.click(getByTitle("Backup options"));
    await user.click(getByText("Restore"));

    expect(await findByText("Backup before restoring?")).toBeInTheDocument();
    await user.click(getByText("Continue"));

    const restoreButton = await findByText("Restore");
    expect(restoreButton).toBeDisabled();

    await user.click(getByRole("checkbox"));
    expect(restoreButton).toBeEnabled();

    await user.click(restoreButton);

    expect(useRestoreFromCloudBackup()).toHaveBeenCalledWith({ id: 1 });
  });

  it("renders in progress", async () => {
    const { findByText } = render(
      <BackupListItem
        backup={backupInProgress}
        restoring={false}
        someBackupInProgress={false}
        someRestoreInProgress={false}
        latestBackupInTargetDeployment={null}
        targetDeployment={{ ...targetDeployment, deploymentType: "dev" }}
        team={team}
        getZipExportUrl={getZipExportUrl}
        canPerformActions
        maxCloudBackups={2}
        progressMessage="In progress message"
      />,
    );

    expect(await findByText("In progress message")).toBeInTheDocument();
  });

  it("calculates in progress", async () => {
    expect(
      progressMessageForBackup(backupInProgress, existingCloudBackupInProgress),
    ).toEqual("progressmsg");

    // backup not in progress
    expect(progressMessageForBackup(backup, null)).toEqual(null);
    expect(
      progressMessageForBackup(backup, existingCloudBackupInProgress),
    ).toEqual(null);

    // existing cloud backup not in progress
    expect(progressMessageForBackup(backupInProgress, null)).toEqual(null);
    expect(
      progressMessageForBackup(backupInProgress, existingCloudBackupRequested),
    ).toEqual(null);

    // id mismatch
    const badIdBackup = {
      ...backupInProgress,
      snapshotId: "bad" as Id<"_exports">,
    };
    expect(
      progressMessageForBackup(badIdBackup, existingCloudBackupInProgress),
    ).toEqual(null);
  });
});
