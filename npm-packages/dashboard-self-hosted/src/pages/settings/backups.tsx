import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { joinUrlPath } from "@common/lib/helpers/joinUrlPath";
import udfs from "@common/udfs";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { Link } from "@ui/Link";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import {
  ArchiveIcon,
  DownloadIcon,
  EnvelopeClosedIcon,
} from "@radix-ui/react-icons";
import { useQuery } from "convex/react";
import { useContext, useId, useState } from "react";

export default function BackupsPage() {
  return (
    <DeploymentSettingsLayout page="backups">
      <BackupsBody />
    </DeploymentSettingsLayout>
  );
}

function BackupsBody() {
  const ctx = useContext(DeploymentInfoContext);
  if (!ctx.ok) {
    return (
      <Sheet>
        <div className="text-content-secondary">
          Loading deployment credentials…
        </div>
      </Sheet>
    );
  }
  return (
    <BackupsLayout deploymentUrl={ctx.deploymentUrl} adminKey={ctx.adminKey} />
  );
}

function BackupsLayout({
  deploymentUrl,
  adminKey,
}: {
  deploymentUrl: string;
  adminKey: string;
}) {
  const existingExport = useQuery(udfs.latestExport.default);
  const [includeStorage, setIncludeStorage] = useState(true);
  const [isRequesting, setIsRequesting] = useState(false);
  const [requestError, setRequestError] = useState<string | null>(null);
  const includeStorageId = useId();

  const inProgress =
    existingExport?.state === "requested" ||
    existingExport?.state === "in_progress";
  const buttonBusy = isRequesting || inProgress;

  const requestBackup = async () => {
    setIsRequesting(true);
    setRequestError(null);
    try {
      const url = joinUrlPath(
        deploymentUrl,
        `/api/export/request/zip?includeStorage=${includeStorage}`,
      ).toString();
      const res = await fetch(url, {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Convex-Client": "dashboard-0.0.0",
        },
      });
      if (!res.ok) {
        const body = await res.text();
        throw new Error(`${res.status}: ${body}`);
      }
    } catch (e) {
      setRequestError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsRequesting(false);
    }
  };

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h3 className="min-w-fit">Backup &amp; Restore</h3>
        <span className="text-sm">
          Trigger a snapshot of this deployment from the dashboard. To restore,
          run{" "}
          <code className="rounded bg-background-tertiary px-1 text-xs">
            npx convex import --replace-all backup.zip
          </code>
          {" — "}
          <Link
            href="https://docs.convex.dev/database/import-export/import#restore-data-from-a-backup-zip-file"
            target="_blank"
          >
            restore docs
          </Link>
          .
        </span>
      </div>

      <div className="scrollbar flex grow flex-col gap-4 overflow-auto pt-1 pl-1 xl:flex-row xl:overflow-hidden">
        <Sheet className="flex h-fit w-full shrink-0 flex-col items-start gap-4 xl:w-60 xl:items-center">
          <Button
            icon={<ArchiveIcon />}
            onClick={requestBackup}
            loading={buttonBusy}
            disabled={buttonBusy}
            className="w-fit"
          >
            Back up now
          </Button>
          <label
            htmlFor={includeStorageId}
            className="ml-px flex items-center gap-2 text-sm"
          >
            <Checkbox
              id={includeStorageId}
              checked={includeStorage}
              onChange={() => setIncludeStorage((v) => !v)}
            />
            Include file storage
          </label>
          <p className="text-xs text-content-secondary">
            Snapshots are stored on this deployment&apos;s configured backend
            storage. Periodic backups are{" "}
            <Link
              href="https://docs.convex.dev/database/backup-restore"
              target="_blank"
            >
              not yet supported
            </Link>{" "}
            on self-hosted; trigger them with cron from your container host.
          </p>
        </Sheet>

        <div className="flex flex-col gap-4 pb-8 xl:grow xl:pb-0">
          {requestError && (
            <Callout variant="error">
              Failed to request backup: {requestError}
            </Callout>
          )}
          <Sheet padding={false} className="flex min-h-72 flex-col">
            <div className="flex items-center justify-between border-b px-4 py-3">
              <h4>Existing Backups</h4>
            </div>
            <BackupsContent
              existingExport={existingExport}
              deploymentUrl={deploymentUrl}
              adminKey={adminKey}
            />
          </Sheet>
        </div>
      </div>
    </div>
  );
}

function BackupsContent({
  existingExport,
  deploymentUrl,
  adminKey,
}: {
  existingExport: ReturnType<typeof useQuery<typeof udfs.latestExport.default>>;
  deploymentUrl: string;
  adminKey: string;
}) {
  if (existingExport === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center py-10">
        <Spinner />
      </div>
    );
  }

  if (existingExport === null) {
    return <BackupsEmptyState message="No backups in this deployment." />;
  }

  if (existingExport.state === "requested") {
    return (
      <BackupsStatusRow
        icon={<Spinner />}
        primary="Backup requested"
        secondary="Waiting to start…"
      />
    );
  }

  if (existingExport.state === "in_progress") {
    return (
      <BackupsStatusRow
        icon={<Spinner />}
        primary="Backup in progress"
        secondary="This usually takes a moment for small deployments."
      />
    );
  }

  if (existingExport.state === "failed") {
    return (
      <div className="p-4">
        <Callout variant="error">
          Latest backup failed. Try again, or report the issue at{" "}
          <Link href="mailto:support@convex.dev">
            <EnvelopeClosedIcon className="mr-0.5 inline" />
            support@convex.dev
          </Link>
          .
        </Callout>
      </div>
    );
  }

  // Completed.
  const completedAt = new Date(
    Number(existingExport.start_ts / BigInt(1_000_000)),
  );
  const expiresAt = new Date(
    Number(existingExport.expiration_ts / BigInt(1_000_000)),
  );
  const isExpired = Date.now() >= expiresAt.getTime();
  if (isExpired) {
    return (
      <BackupsEmptyState message="The most recent backup has expired. Create a new one to download." />
    );
  }

  const params = new URLSearchParams({ adminKey });
  const downloadHref = joinUrlPath(
    deploymentUrl,
    `/api/export/zip/${existingExport._id}?${params.toString()}`,
  ).toString();
  const filename = `convex-backup-${existingExport.start_ts.toString()}.zip`;

  return (
    <div className="divide-y">
      <div className="flex items-center justify-between gap-4 px-4 py-3">
        <div className="flex min-w-0 flex-col">
          <div className="truncate font-mono text-sm">{filename}</div>
          <div className="text-xs text-content-secondary">
            Created {completedAt.toLocaleString()} · Expires{" "}
            {expiresAt.toLocaleString()}
          </div>
        </div>
        <Button
          size="sm"
          variant="primary"
          inline
          aria-label="Download backup"
          href={downloadHref}
          icon={<DownloadIcon />}
        >
          <span className="hidden md:flex">Download</span>
        </Button>
      </div>
    </div>
  );
}

function BackupsStatusRow({
  icon,
  primary,
  secondary,
}: {
  icon: React.ReactNode;
  primary: string;
  secondary?: string;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 py-10">
      <div className="text-content-secondary">{icon}</div>
      <div className="text-sm text-content-primary">{primary}</div>
      {secondary && (
        <div className="text-xs text-content-secondary">{secondary}</div>
      )}
    </div>
  );
}

function BackupsEmptyState({ message }: { message: string }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 py-10 text-center">
      <div className="rounded-md bg-purple-100/40 p-2 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300">
        <ArchiveIcon className="size-5" />
      </div>
      <div className="text-content-primary">{message}</div>
      <div className="max-w-sm text-xs text-content-secondary">
        Use the <span className="font-medium">Back up now</span> button on the
        left to create a downloadable snapshot.
      </div>
    </div>
  );
}
