import {
  Button,
  Tooltip,
  Spinner,
  TimestampDistance,
  toast,
  Sheet,
  Callout,
  LocalDevCallout,
  Checkbox,
  TextInput,
} from "dashboard-common";
import { Popover } from "elements/Popover";
import {
  useDisablePeriodicBackup,
  useGetPeriodicBackupConfig,
  useConfigurePeriodicBackup,
} from "api/backups";
import { useCurrentProject } from "api/projects";
import { useId, useState } from "react";
import {
  DeploymentResponse,
  Team,
  TeamEntitlementsResponse,
} from "generatedApi";
import Link from "next/link";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { useHasProjectAdminPermissions } from "api/roles";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { BackupList } from "./BackupList";
import { BackupRestoreStatus } from "./BackupRestoreStatus";
import { BackupNowButton } from "./BackupListItem";

export function Backups({
  team,
  deployment,
  entitlements,
}: {
  team: Team;
  deployment: DeploymentResponse;
  entitlements: TeamEntitlementsResponse;
}) {
  const project = useCurrentProject();

  const existingExport = useQuery(udfs.latestExport.default);
  const periodicBackupsEnabled = entitlements.periodicBackupsEnabled ?? false;
  const maxCloudBackups = entitlements.maxCloudBackups ?? 2;

  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment.projectId,
  );
  const canPerformActions =
    deployment.deploymentType !== "prod" || hasAdminPermissions;

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-4">
        <h3 className="min-w-fit">Backup & Restore</h3>
        <span className="text-sm">
          Use this page to automatically or manually backup and restore your
          deployment data.{" "}
          <Link
            href="https://docs.convex.dev/database/backup-restore"
            className="text-content-link dark:underline"
          >
            Learn more
          </Link>
        </span>
      </div>
      <div className="flex grow flex-col gap-8 overflow-auto pl-1 pt-1 scrollbar lg:flex-row lg:overflow-hidden">
        <div className="flex shrink-0 flex-col lg:w-60">
          {periodicBackupsEnabled ? (
            <AutomaticBackupSelector
              deployment={deployment}
              canPerformActions={canPerformActions}
            />
          ) : (
            <Tooltip
              tip="Automatic backups are only available on paid plans."
              className="flex gap-1"
            >
              <label className="flex cursor-not-allowed items-center gap-2 text-sm">
                <Checkbox disabled checked={false} onChange={() => {}} />
                Backup Automatically
              </label>
              <span
                className="rounded bg-util-accent px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wider text-white"
                title="Only available in paid plans"
              >
                Pro
              </span>
            </Tooltip>
          )}

          <hr className="my-6 w-full" />

          <BackupNowButton
            deployment={deployment}
            team={team}
            maxCloudBackups={maxCloudBackups}
            canPerformActions={canPerformActions}
          />
          <BackupProCallouts
            team={team}
            periodicBackupsEnabled={periodicBackupsEnabled}
            maxCloudBackups={maxCloudBackups}
          />
        </div>

        <div className="flex flex-col gap-4 pb-8 lg:grow lg:pb-0">
          {existingExport &&
            existingExport._creationTime < new Date("2024-11-15").getTime() &&
            existingExport.state === "completed" &&
            Date.now() <
              Number(existingExport.expiration_ts / BigInt(1000000)) && (
              <Callout>
                <div>
                  Looking for your last Snapshot Export? You can now use Cloud
                  Backups to backup and restore your deployment data. Download
                  your last snapshot{" "}
                  <Link
                    href={`/t/${team.slug}/${project?.slug}/${deployment.name}/settings/snapshots`}
                    className="text-content-link hover:underline dark:underline"
                  >
                    here
                  </Link>
                  .
                </div>
              </Callout>
            )}
          <BackupRestoreStatus deployment={deployment} team={team} />

          <Sheet padding={false} className="min-h-72">
            <BackupList
              team={team}
              targetDeployment={deployment}
              canPerformActions={canPerformActions}
              maxCloudBackups={maxCloudBackups}
            />
          </Sheet>
        </div>
      </div>
    </div>
  );
}

function BackupProCallouts({
  team,
  periodicBackupsEnabled,
  maxCloudBackups,
}: {
  team: Team;
  periodicBackupsEnabled: boolean;
  maxCloudBackups: number;
}) {
  return (
    <>
      {!periodicBackupsEnabled && (
        <LocalDevCallout
          className="mt-6 flex-col"
          tipText="Tip: Run this to enable automatic backups locally:"
          command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement periodic_backups_enabled --team-id ${team?.id} --reason "local" true --for-real`}
        />
      )}
      {maxCloudBackups <= 2 && (
        <LocalDevCallout
          className="mt-6 flex-col"
          tipText="Tip: Run this to increase the backup limit locally:"
          command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement max_cloud_backups --team-id ${team?.id} --reason "local" 50 --for-real`}
        />
      )}
    </>
  );
}

function AutomaticBackupSelector({
  deployment,
  canPerformActions,
}: {
  deployment: DeploymentResponse;
  canPerformActions: boolean;
}) {
  const periodicBackup = useGetPeriodicBackupConfig(deployment.id);
  const configurePeriodicBackup = useConfigurePeriodicBackup(deployment.id);
  const disablePeriodicBackup = useDisablePeriodicBackup(deployment.id);

  const [isSubmitting, setIsSubmitting] = useState(false);

  return (
    <Tooltip
      tip={
        !canPerformActions
          ? "You do not have permission to change the backup settings in production."
          : undefined
      }
    >
      <div className="flex flex-col gap-2">
        <label className="mb-1 flex items-center gap-2 text-sm">
          <Checkbox
            checked={!!periodicBackup}
            disabled={
              periodicBackup === undefined || isSubmitting || !canPerformActions
            }
            onChange={async () => {
              setIsSubmitting(true);
              try {
                if (periodicBackup === null) {
                  // Enable automatic backups
                  const defaultCronspec = "0 0 * * *";
                  await configurePeriodicBackup({ cronspec: defaultCronspec });
                } else {
                  // Disable automatic backups
                  await disablePeriodicBackup();
                }
              } finally {
                setIsSubmitting(false);
              }
            }}
          />
          Backup automatically{" "}
          {isSubmitting && (
            <div>
              <Spinner />
            </div>
          )}
        </label>
        {periodicBackup && (
          <>
            <BackupScheduleSelector
              cronspec={periodicBackup.cronspec}
              deployment={deployment}
              disabled={!canPerformActions}
            />
            <div>
              <TimestampDistance
                prefix="Next backup "
                date={new Date(periodicBackup.nextRun)}
              />
              <p className="text-xs text-content-secondary">
                ({new Date(periodicBackup.nextRun).toLocaleString()}{" "}
                {localTimezoneName()})
              </p>
            </div>
          </>
        )}
      </div>
    </Tooltip>
  );
}

export function BackupScheduleSelector({
  cronspec,
  deployment,
  disabled,
}: {
  cronspec: string;
  deployment: DeploymentResponse;
  disabled: boolean;
}) {
  const [minutesUtc, hoursUtc] = cronspec.split(" ");

  const date = new Date();
  date.setUTCHours(+hoursUtc, +minutesUtc);

  return (
    <Popover
      button={
        <Button
          variant="neutral"
          className="relative w-full pl-3 pr-10 font-normal"
          disabled={disabled}
        >
          <span className="flex flex-col truncate">
            Daily at{" "}
            {new Intl.DateTimeFormat(undefined, {
              hour: "2-digit",
              minute: "2-digit",
            }).format(date)}
          </span>
          <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
            <ChevronDownIcon
              className="h-5 w-5 text-content-tertiary"
              aria-hidden="true"
            />
          </span>
        </Button>
      }
      openButtonClassName="*:bg-background-tertiary"
    >
      {({ close }) => (
        <BackupScheduleSelectorInner
          defaultValue={date}
          onClose={close}
          deployment={deployment}
        />
      )}
    </Popover>
  );
}

export function BackupScheduleSelectorInner({
  defaultValue,
  onClose,
  deployment,
}: {
  defaultValue: Date;
  onClose: () => void;
  deployment: DeploymentResponse;
}) {
  const configurePeriodicBackup = useConfigurePeriodicBackup(deployment.id);

  const initialValue = `${defaultValue.getHours().toString().padStart(2, "0")}:${defaultValue.getMinutes().toString().padStart(2, "0")}`;
  const [value, setValue] = useState(initialValue);

  const id = useId();

  const [isSubmitting, setIsSubmitting] = useState(false);

  return (
    <form
      className="flex min-w-72 flex-col items-end gap-3"
      onSubmit={async (e) => {
        e.preventDefault();

        const [newHoursLocal, newMinutesLocal] = value.split(":");
        const newDate = new Date();
        newDate.setHours(+newHoursLocal, +newMinutesLocal);

        setIsSubmitting(true);
        try {
          await configurePeriodicBackup({
            cronspec: `${newDate.getUTCMinutes()} ${newDate.getUTCHours()} * * *`,
          });
        } finally {
          setIsSubmitting(false);
        }

        toast("success", "Your backup schedule was modified.");

        onClose();
      }}
    >
      <TextInput
        id={id}
        type="time"
        label={`Time (${localTimezoneName()})`}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        required
      />
      <Button
        type="submit"
        disabled={value === initialValue || isSubmitting}
        icon={isSubmitting ? <Spinner /> : undefined}
      >
        Change
      </Button>
    </form>
  );
}

function localTimezoneName(): string {
  return new Intl.DateTimeFormat(undefined, {
    timeZoneName: "short",
  })
    .formatToParts(new Date())
    .find((part) => part.type === "timeZoneName")!.value;
}
