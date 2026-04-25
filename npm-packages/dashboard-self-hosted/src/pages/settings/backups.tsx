import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { Link } from "@ui/Link";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import udfs from "@common/udfs";
import { joinUrlPath } from "@common/lib/helpers/joinUrlPath";
import {
  ArchiveIcon,
  DownloadIcon,
  EnvelopeClosedIcon,
} from "@radix-ui/react-icons";
import { useContext, useId, useState } from "react";
import { useQuery } from "convex/react";

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
    <BackupsCard deploymentUrl={ctx.deploymentUrl} adminKey={ctx.adminKey} />
  );
}

function BackupsCard({
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

  const inFlight =
    existingExport?.state === "requested" ||
    existingExport?.state === "in_progress" ||
    isRequesting;

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
    <Sheet>
      <div className="mb-2 flex w-full items-center justify-between gap-4">
        <h3>Backup &amp; Restore</h3>
        <Button
          icon={<ArchiveIcon />}
          onClick={requestBackup}
          loading={inFlight}
          disabled={inFlight}
        >
          {inFlight ? "Back up in progress" : "Back up now"}
        </Button>
      </div>
      <p className="mb-3 max-w-prose text-content-primary">
        Trigger an immediate snapshot of this deployment. The resulting{" "}
        <code className="rounded bg-background-tertiary px-1 text-xs">
          .zip
        </code>{" "}
        is a portable backup you can download here. To restore from a backup,
        run{" "}
        <code className="rounded bg-background-tertiary px-1 text-xs">
          npx convex import --replace-all backup.zip
        </code>{" "}
        from your project — see the{" "}
        <Link
          href="https://docs.convex.dev/database/import-export/import#restore-data-from-a-backup-zip-file"
          target="_blank"
        >
          restore docs
        </Link>
        .
      </p>

      <label
        htmlFor={includeStorageId}
        className="mb-4 ml-px inline-flex items-center gap-2 text-sm"
      >
        <Checkbox
          id={includeStorageId}
          checked={includeStorage}
          onChange={() => setIncludeStorage((v) => !v)}
        />
        Include file storage in the backup
      </label>

      {requestError && (
        <Callout variant="error" className="mb-3">
          Failed to request backup: {requestError}
        </Callout>
      )}

      <LatestExport
        existingExport={existingExport}
        deploymentUrl={deploymentUrl}
        adminKey={adminKey}
      />
    </Sheet>
  );
}

function LatestExport({
  existingExport,
  deploymentUrl,
  adminKey,
}: {
  existingExport: ReturnType<typeof useQuery<typeof udfs.latestExport.default>>;
  deploymentUrl: string;
  adminKey: string;
}) {
  if (existingExport === undefined) {
    return <Spinner />;
  }
  if (existingExport === null) {
    return (
      <span className="text-content-secondary">
        No snapshot has been requested yet.
      </span>
    );
  }
  if (existingExport.state === "requested") {
    return (
      <div className="flex items-center gap-2 text-content-primary">
        <Spinner /> Backup requested, waiting to start…
      </div>
    );
  }
  if (existingExport.state === "in_progress") {
    return (
      <div className="flex items-center gap-2 text-content-primary">
        <Spinner /> Backup in progress…
      </div>
    );
  }
  if (existingExport.state === "failed") {
    return (
      <Callout variant="error">
        Latest backup failed. Try again, or report the issue at{" "}
        <Link href="mailto:support@convex.dev">
          <EnvelopeClosedIcon className="mr-0.5 inline" />
          support@convex.dev
        </Link>
        .
      </Callout>
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
      <span className="text-content-secondary">
        Latest backup has expired. Create a new one above.
      </span>
    );
  }

  const params = new URLSearchParams({ adminKey });
  const downloadHref = joinUrlPath(
    deploymentUrl,
    `/api/export/zip/${existingExport._id}?${params.toString()}`,
  ).toString();
  const filename = `convex-backup-${existingExport.start_ts.toString()}.zip`;

  return (
    <div className="max-w-2xl rounded-md border">
      <div className="rounded-t-md bg-background-primary px-4 py-2 text-sm sm:flex sm:justify-between sm:gap-4">
        <div className="truncate">Created {completedAt.toLocaleString()}</div>
        <div className="truncate text-content-errorSecondary">
          Expires {expiresAt.toLocaleString()}
        </div>
      </div>
      <div className="flex items-center gap-x-4 px-4 py-2">
        <div className="flex-1 truncate font-mono text-sm">{filename}</div>
        <Button
          size="sm"
          variant="primary"
          inline
          aria-label="Download backup"
          href={downloadHref}
        >
          <DownloadIcon aria-label="Download" />
          <span className="hidden md:flex">Download</span>
        </Button>
      </div>
    </div>
  );
}
