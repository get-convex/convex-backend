import { Disclosure } from "@headlessui/react";
import {
  ChevronUpIcon,
  ChevronDownIcon,
  QuestionMarkCircledIcon,
  ArrowRightIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { stringifyValue } from "@common/lib/stringifyValue";
import {
  Team,
  MemberResponse,
  ProjectDetails,
  AuditLogAction,
  DeploymentResponse,
  AuditLogEventResponse,
} from "generatedApi";
import { captureMessage } from "@sentry/nextjs";
import startCase from "lodash/startCase";
import Link from "next/link";
import { useDeploymentById } from "api/deployments";
import { BackupIdentifier } from "elements/BackupIdentifier";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { Tooltip } from "@ui/Tooltip";
import { formatUsd } from "@common/lib/utils";

// TODO: Figure out how to get typing on metadata in
// big brain
type AuditLogEntryMetadata = {
  noun?: string;
  previous?: Record<string, any> | null;
  current?: Record<string, any> | null;
};

export function AuditLogItem({
  entry,
  team,
  memberId,
  members,
  projects,
}: {
  entry: AuditLogEventResponse;
  team: Team;
  memberId: number | null;
  members: MemberResponse[];
  projects: ProjectDetails[];
}) {
  return (
    <Disclosure>
      {({ open }) => (
        <div
          className="border-b px-6 py-2 last:border-b-0"
          data-testid="audit-log-item"
        >
          <div className="grid grid-cols-[9fr_3fr]">
            <span className="text-sm">
              <AuditLogItemActor
                entry={entry}
                memberId={memberId}
                members={members}
              />{" "}
              <EntryAction
                action={entry.action}
                metadata={entry.metadata as AuditLogEntryMetadata}
                team={team}
                members={members}
                projects={projects}
              />
            </span>
            <span className="ml-auto flex gap-1">
              <TimestampDistance date={new Date(entry.createTime)} />
              <Disclosure.Button
                as={Button}
                inline
                variant="neutral"
                size="xs"
                tipSide="left"
                tip="View entry metadata"
              >
                {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
              </Disclosure.Button>
            </span>
          </div>
          <Disclosure.Panel>
            <ReadonlyCode
              height={{
                type: "content",
              }}
              disableLineNumbers
              code={stringifyValue(
                JSON.stringify(entry.metadata, undefined, 2),
                true,
              ).slice(1, -1)}
              path={`${entry.createTime}`}
            />
          </Disclosure.Panel>
        </div>
      )}
    </Disclosure>
  );
}

function EntryAction({
  action,
  metadata,
  team,
  members,
  projects,
}: {
  action: AuditLogAction;
  metadata: AuditLogEntryMetadata;
  team: Team;
  members: MemberResponse[];
  projects: ProjectDetails[];
}) {
  switch (action) {
    case "createProject":
    case "updateProject":
    case "deleteProject":
      return (
        <ProjectEntryAction
          projects={projects}
          team={team}
          action={action}
          metadata={metadata}
        />
      );
    case "receiveProject":
      return (
        <span>
          transferred project{" "}
          <ProjectLink
            projectId={metadata.current?.id}
            metadata={metadata}
            projects={projects}
            team={team}
          />{" "}
          to this team.
        </span>
      );
    case "transferProject":
      return (
        <span>
          transferred project{" "}
          <ProjectLink
            projectId={metadata.previous?.id}
            metadata={metadata}
            projects={projects}
            team={team}
          />{" "}
          to another team.
        </span>
      );
    case "updateBillingContact":
      return <span>updated the billing contact</span>;
    case "updateBillingAddress":
      return <span>updated the billing address</span>;
    case "updatePaymentMethod":
      return <span>updated the payment method</span>;
    case "removeMember":
      if (!metadata.previous?.email) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          removed{" "}
          <span className="font-semibold">{metadata.previous.email}</span> from
          the team
        </span>
      );
    case "inviteMember":
      if (!metadata.current?.email) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          invited{" "}
          <span className="font-semibold">{metadata.current.email}</span> to the
          team
        </span>
      );
    case "cancelMemberInvitation":
      if (!metadata.previous?.email) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          canceled the team invitation for{" "}
          <span className="font-semibold">{metadata.previous.email}</span>
        </span>
      );
    case "updateMemberRole":
      if (
        !metadata.current?.role ||
        !metadata.current?.id ||
        (!metadata.current?.name && !metadata.current?.email)
      ) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          changed{" "}
          <TeamMemberLink
            memberId={metadata.current.id}
            name={metadata.current.name || metadata.current.email}
          />
          's role to{" "}
          <span className="font-semibold">
            {startCase(metadata.current.role)}
          </span>
        </span>
      );
    case "updateMemberProjectRole":
      if (
        metadata.current &&
        metadata.current.project_id &&
        metadata.current.role &&
        metadata.current.member_id
      ) {
        return (
          <ProjectRoleUpdateEntry
            role={metadata.current.role}
            members={members}
            memberId={metadata.current.member_id}
            projectId={metadata.current.project_id}
            projects={projects}
            team={team}
          />
        );
      }

      if (
        metadata.previous &&
        metadata.previous.project_id &&
        metadata.previous.role &&
        metadata.previous.member_id
      ) {
        return (
          <ProjectRoleUpdateEntry
            role={metadata.previous.role}
            members={members}
            memberId={metadata.previous.member_id}
            projectId={metadata.previous.project_id}
            projects={projects}
            team={team}
            removed
          />
        );
      }

      captureMessage(`Found malformed metadata for ${action}`, "error");
      return <UnhandledAction action={action} />;

    case "joinTeam":
      return <span>joined the team</span>;
    case "createTeam":
      return <span>created the team</span>;
    case "updateTeam":
      return <span>updated the team</span>;
    case "deleteTeam":
      return <span>deleted the team</span>;
    case "createDeployment":
      if (!metadata.current?.type || !metadata.current?.projectId) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }

      return (
        <span>
          created a{" "}
          <span className="font-semibold">{metadata.current.type}</span>{" "}
          deployment for{" "}
          <ProjectLink
            projectId={metadata.current.projectId}
            metadata={metadata}
            projects={projects}
            team={team}
          />
        </span>
      );
    case "deleteDeployment":
      if (!metadata.previous?.type || !metadata.previous?.projectId) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          deleted a{" "}
          <span className="font-semibold">{metadata.previous.type}</span>{" "}
          deployment for{" "}
          <ProjectLink
            projectId={metadata.previous.projectId}
            metadata={metadata}
            projects={projects}
            team={team}
          />
        </span>
      );
    case "createProjectEnvironmentVariable":
    case "updateProjectEnvironmentVariable":
    case "deleteProjectEnvironmentVariable":
      return (
        <EnvironmentVariableEntryAction
          action={action}
          metadata={metadata}
          projects={projects}
          team={team}
        />
      );
    case "createSubscription":
      return (
        <span>subscribed to {metadata.current?.plan || "a Convex plan"}</span>
      );
    case "cancelSubscription":
      return (
        <span>
          canceled the {metadata.previous?.plan || "Convex"} subscription
        </span>
      );
    case "resumeSubscription":
      return (
        <span>
          resumed the {metadata.current?.plan || "Convex"} subscription
        </span>
      );
    case "changeSubscriptionPlan":
      if (!metadata.previous?.plan || !metadata.current?.plan) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          changed the subscription plan from{" "}
          <span className="font-semibold">{metadata.previous?.plan}</span>·to{" "}
          <span className="font-semibold">{metadata.current?.plan}</span>
        </span>
      );
    case "createCustomDomain":
      return (
        <span>
          added {metadata.current?.domain ? "the" : "a"} custom domain{" "}
          {metadata.current?.domain && (
            <span className="font-semibold">{metadata.current.domain} </span>
          )}
          {metadata.current?.projectId && (
            <span>
              for{" "}
              <ProjectLink
                projectId={metadata.current?.projectId}
                metadata={metadata}
                projects={projects}
                team={team}
              />
            </span>
          )}
        </span>
      );
    case "deleteCustomDomain":
      return (
        <span>
          deleted {metadata.previous?.domain ? "the" : "a"} custom domain{" "}
          {metadata.previous?.domain && (
            <span className="font-semibold">{metadata.previous?.domain} </span>
          )}
          {metadata.previous?.projectId && (
            <span>
              for{" "}
              <ProjectLink
                projectId={metadata.previous?.projectId}
                metadata={metadata}
                projects={projects}
                team={team}
              />
            </span>
          )}
        </span>
      );
    case "createTeamAccessToken":
      return (
        <span>
          {metadata.current && (
            <AccessTokenSettingsLink
              team={team}
              projects={projects}
              metadataEntity={metadata.current}
              verb="created"
            />
          )}
        </span>
      );
    case "viewTeamAccessToken":
      // we expect these to never be logged
      captureMessage("Found viewTeamAccessToken audit log", "error");
      return (
        <span>
          {metadata.current && (
            <AccessTokenSettingsLink
              team={team}
              projects={projects}
              metadataEntity={metadata.current}
              verb="viewed"
            />
          )}
        </span>
      );
    case "updateTeamAccessToken":
      return (
        <span>
          {metadata.current && (
            <AccessTokenSettingsLink
              team={team}
              projects={projects}
              metadataEntity={metadata.current}
              verb="updated"
            />
          )}
        </span>
      );
    case "deleteTeamAccessToken":
      return (
        <span>
          {metadata.previous && (
            <AccessTokenSettingsLink
              team={team}
              projects={projects}
              metadataEntity={metadata.previous}
              verb="deleted"
            />
          )}
        </span>
      );
    case "startManualCloudBackup":
    case "deleteCloudBackup": {
      const verb =
        metadata.previous && metadata.current
          ? "updated"
          : metadata.previous
            ? "deleted"
            : "requested";
      const deploymentId =
        metadata.current?.sourceDeploymentId ||
        metadata.previous?.sourceDeploymentId;
      if (!deploymentId) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          {verb} a backup of{" "}
          <DeploymentSettingsLink
            projects={projects}
            team={team}
            deploymentId={deploymentId}
            urlSuffix="/backups"
          />
        </span>
      );
    }
    case "restoreFromCloudBackup":
      if (
        !metadata.current?.targetDeploymentId ||
        !metadata.current?.backup ||
        !metadata.current?.backup?.sourceDeploymentId ||
        !metadata.current?.backup?.requestedTime
      ) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      return (
        <span>
          restored into{" "}
          <DeploymentSettingsLink
            projects={projects}
            team={team}
            deploymentId={metadata.current?.targetDeploymentId}
            urlSuffix="/backups"
          />{" "}
          from the backup <BackupIdentifier backup={metadata.current?.backup} />
        </span>
      );
    case "configurePeriodicBackup":
    case "disablePeriodicBackup": {
      if (
        !metadata.current?.sourceDeploymentId &&
        !metadata.previous?.sourceDeploymentId
      ) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }
      const verb =
        metadata.previous && metadata.current
          ? "updated"
          : metadata.previous
            ? "deleted"
            : "created";
      return (
        <span>
          {verb} a periodic backup schedule for{" "}
          <DeploymentSettingsLink
            projects={projects}
            team={team}
            deploymentId={
              metadata.current?.sourceDeploymentId ||
              metadata.previous?.sourceDeploymentId
            }
            urlSuffix="/backups"
          />{" "}
        </span>
      );
    }
    case "applyReferralCode": {
      return <span>applied a referral code</span>;
    }
    case "disableTeamExceedingSpendingLimits": {
      return (
        <span>
          disabled your team's projects due to exceeding spending limits
        </span>
      );
    }
    case "setSpendingLimit": {
      const { previous, current } = metadata;
      if (
        !isValidSpendingLimitDiff(previous) ||
        !isValidSpendingLimitDiff(current)
      ) {
        captureMessage(`Found malformed metadata for ${action}`, "error");
        return <UnhandledAction action={action} />;
      }

      return (
        <>
          <span>made a change to the spending limits:</span>
          <br />
          <div className="mt-2 inline-grid grid-cols-[auto_auto_auto_auto] items-center gap-x-2 gap-y-1.5">
            {previous.warningThresholdCents !==
              current.warningThresholdCents && (
              <SpendingLimitLine
                label="Warning threshold"
                tooltip="When your team exceeds this spending level, team admins will be notified."
                previousValue={previous.warningThresholdCents}
                currentValue={current.warningThresholdCents}
              />
            )}
            {previous.disableThresholdCents !==
              current.disableThresholdCents && (
              <SpendingLimitLine
                label="Disable threshold"
                tooltip="When your team exceeds this spending level, your projects will be disabled."
                previousValue={previous.disableThresholdCents}
                currentValue={current.disableThresholdCents}
              />
            )}
          </div>
        </>
      );
    }
    case "verifyOAuthApplication": {
      return <span>verified an OAuth application</span>;
    }
    case "deleteOAuthApplication": {
      return <span>deleted an OAuth application</span>;
    }
    case "createOAuthApplication": {
      return <span>created an OAuth application</span>;
    }
    case "updateOAuthApplication": {
      return <span>updated an OAuth application</span>;
    }
    case "generateOAuthClientSecret": {
      return <span>generated a client secret for an OAuth application</span>;
    }
    case "createWorkosTeam": {
      return <span>created a WorkOS team</span>;
    }
    case "createWorkosEnvironment": {
      return <span>created a WorkOS environment</span>;
    }
    case "retrieveWorkosEnvironmentCredentials": {
      return <span>retrieve WorkOS Environment credentials</span>;
    }
    case "enableSSO": {
      return <span>enabled SSO</span>;
    }
    case "disableSSO": {
      return <span>disabled SSO</span>;
    }
    default:
      action satisfies never;
      captureMessage(`Unhandled audit log action: ${action}`, "error");
      return <UnhandledAction action={action} />;
  }
}

export function ProjectLink({
  metadata,
  projects,
  team,
  projectId,
}: {
  projectId: number;
  metadata: AuditLogEntryMetadata;
  projects: ProjectDetails[];
  team: Team;
}) {
  const project = projects.find((p) => p.id === projectId);

  const projectName =
    project?.name ||
    (metadata.noun === "project"
      ? metadata.current?.name || metadata.previous?.name
      : "a deleted project");

  return project ? (
    <Link
      href={`/t/${team.slug}/${project.slug}/settings`}
      className="font-semibold text-content-link hover:underline"
      target="_blank"
    >
      {projectName}
    </Link>
  ) : (
    <span className="font-semibold text-content-secondary">{projectName}</span>
  );
}

function EnvironmentVariableEntryAction({
  projects,
  team,
  action,
  metadata,
}: {
  projects: ProjectDetails[];
  team: Team;
  action: string;
  metadata: AuditLogEntryMetadata;
}) {
  if (!metadata.current?.projectId && !metadata.previous?.projectId) {
    captureMessage(`Found malformed metadata for ${action}`, "error");
    return <UnhandledAction action={action} />;
  }
  const verb =
    metadata.previous && metadata.current
      ? "updated"
      : metadata.previous
        ? "deleted"
        : "created";

  const variableName = metadata.current?.name || metadata.previous?.name;

  if (!variableName) {
    captureMessage(
      `Could not find variable name in metadata for ${action}`,
      "error",
    );
    return <UnhandledAction action={action} />;
  }

  return (
    <span>
      {verb} the environment variable{" "}
      <span className="font-semibold">{variableName}</span> in{" "}
      <ProjectLink
        projectId={metadata.current?.projectId || metadata.previous?.projectId}
        metadata={metadata}
        projects={projects}
        team={team}
      />
    </span>
  );
}

function ProjectEntryAction({
  projects,
  team,
  action,
  metadata,
}: {
  projects: ProjectDetails[];
  team: Team;
  action: string;
  metadata: AuditLogEntryMetadata;
}) {
  if (!metadata.current?.id && !metadata.previous?.id) {
    captureMessage(`Found malformed metadata for ${action}`, "error");
    return <UnhandledAction action={action} />;
  }

  const verb =
    metadata.previous && metadata.current
      ? "updated"
      : metadata.previous
        ? "deleted"
        : "created";

  return (
    <span>
      {verb} the project{" "}
      <ProjectLink
        projectId={metadata.current?.id || metadata.previous?.id}
        metadata={metadata}
        projects={projects}
        team={team}
      />
    </span>
  );
}

function UnhandledAction({ action }: { action: string }) {
  return (
    <span>
      performed the action{" "}
      <span className="font-semibold">{startCase(action)}</span>
    </span>
  );
}

function ProjectRoleUpdateEntry({
  role,
  members,
  memberId,
  projectId,
  projects,
  team,
  removed = false,
}: {
  members: MemberResponse[];
  role?: "admin";
  memberId: number;
  projectId: number;
  projects: ProjectDetails[];
  team: Team;
  removed?: boolean;
}) {
  return (
    <span>
      {removed ? "removed" : "gave"} the{" "}
      <span className="font-semibold">Project {startCase(role)}</span> role{" "}
      {removed ? "from" : "to"}{" "}
      <TeamMemberLink
        memberId={memberId}
        name={
          members.find((m) => m.id === memberId)?.name || memberId.toString()
        }
      />{" "}
      for{" "}
      <ProjectLink
        projectId={projectId}
        // Don't need metadata for this project link
        metadata={{}}
        projects={projects}
        team={team}
      />
    </span>
  );
}

function AuditLogItemActor({
  entry,
  memberId,
  members,
}: {
  entry: AuditLogEventResponse;
  memberId: number | null;
  members: MemberResponse[];
}) {
  if (entry.actor === "system") {
    return <span className="font-semibold">Convex</span>;
  }
  if ("team" in entry.actor) {
    return <span className="font-semibold">A Deploy Key</span>;
  }
  const member = members?.find((m) => m.id === memberId);
  return member ? (
    <TeamMemberLink memberId={member.id} name={member.name || member.email} />
  ) : (entry.metadata as AuditLogEntryMetadata).noun === "member" ? (
    <span className="font-semibold">
      {(entry.metadata as AuditLogEntryMetadata)?.current?.email ||
        (entry.metadata as AuditLogEntryMetadata)?.previous?.email}
    </span>
  ) : (
    <span className="font-semibold">
      A team member{" "}
      <span className="font-normal text-content-secondary">
        (Member ID: {memberId})
      </span>
    </span>
  );
}

function deploymentDisplayName(deployment: DeploymentResponse) {
  switch (deployment.deploymentType) {
    case "prod":
      return "the production deployment";
    case "dev":
      return "a development deployment";
    case "preview":
      return "a preview deployment";
    default:
      return "a deployment";
  }
}
function DeploymentSettingsLink({
  projects,
  team,
  deploymentId,
  urlSuffix = "",
}: {
  projects: ProjectDetails[];
  team: Team;
  deploymentId: number;
  urlSuffix?: string;
}) {
  const deployment = useDeploymentById(team.id, deploymentId);
  if (!deployment) {
    return <span>a deployment</span>;
  }

  const project = projects.find((p) => p.id === deployment.projectId);
  if (!project) {
    captureMessage(
      `Malformed deploy key audit log entry:
      deployment ${deploymentId} has project id ${deployment.projectId}
      which is not found within the projects of team ${team.id}`,
      "error",
    );
    return <span>a deployment</span>;
  }

  return (
    <>
      <Link
        href={`/t/${team.slug}/${project.slug}/${deployment.name}/settings${urlSuffix}`}
        className="font-semibold text-content-link hover:underline"
        target="_blank"
      >
        {deploymentDisplayName(deployment)}
      </Link>
      <span> of {project.name}</span>
    </>
  );
}

function ProjectSettingsLink({
  projects,
  team,
  projectId,
}: {
  projects: ProjectDetails[];
  team: Team;
  projectId: number;
}) {
  const project = projects.find((p) => p.id === projectId);
  if (!project) {
    return <span>Project {projectId}</span>;
  }

  return (
    <Link
      href={`/t/${team.slug}/${project.slug}/settings`}
      className="font-semibold text-content-link hover:underline"
      target="_blank"
    >
      {project.name}
    </Link>
  );
}

function AccessTokenSettingsLink({
  team,
  projects,
  metadataEntity,
  verb,
}: {
  team: Team;
  projects: ProjectDetails[];
  metadataEntity: Record<string, any>;
  verb: string;
}) {
  return (
    <>
      {verb} the deploy key{" "}
      <span className="font-semibold">{metadataEntity.name}</span>
      {metadataEntity.deploymentId && (
        <>
          {" "}
          in{" "}
          <DeploymentSettingsLink
            projects={projects}
            team={team}
            deploymentId={metadataEntity?.deploymentId}
          />
        </>
      )}
      {metadataEntity.projectId && (
        <>
          {" "}
          in{" "}
          <ProjectSettingsLink
            projects={projects}
            team={team}
            projectId={metadataEntity?.projectId}
          />
        </>
      )}
    </>
  );
}

type SpendingLimitDiff = {
  warningThresholdCents: number | null;
  disableThresholdCents: number | null;
};

function isValidSpendingLimitDiff(value: unknown): value is SpendingLimitDiff {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  return (
    "warningThresholdCents" in value &&
    "disableThresholdCents" in value &&
    (typeof value.warningThresholdCents === "number" ||
      value.warningThresholdCents === null) &&
    (typeof value.disableThresholdCents === "number" ||
      value.disableThresholdCents === null)
  );
}

function SpendingLimitLine({
  label,
  tooltip,
  previousValue,
  currentValue,
}: {
  label: string;
  tooltip: string;
  previousValue: number | null;
  currentValue: number | null;
}) {
  return (
    <div className="contents">
      <header className="mr-2 flex items-center gap-1">
        <div className="text-content-secondary">{label}</div>
        <Tooltip tip={tooltip} side="top">
          <QuestionMarkCircledIcon className="text-content-tertiary" />
        </Tooltip>
      </header>
      <SpendingValue valueCents={previousValue} />
      <ArrowRightIcon className="text-content-tertiary" />
      <SpendingValue valueCents={currentValue} />
    </div>
  );
}

function SpendingValue({ valueCents }: { valueCents: number | null }) {
  return (
    <div className="text-right font-medium text-content-primary tabular-nums">
      {valueCents === null ? "None" : formatUsd(valueCents / 100)}
    </div>
  );
}
