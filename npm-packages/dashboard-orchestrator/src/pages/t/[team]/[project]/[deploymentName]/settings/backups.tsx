import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { joinUrlPath } from "@common/lib/helpers/joinUrlPath";
import udfs from "@common/udfs";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Link } from "@ui/Link";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { Menu, MenuItem } from "@ui/Menu";
import {
  ArchiveIcon,
  DotsVerticalIcon,
  DownloadIcon,
  EnvelopeClosedIcon,
} from "@radix-ui/react-icons";
import { useQuery } from "convex/react";
import { useContext, useId, useState } from "react";

type ExportRow = NonNullable<
  ReturnType<typeof useQuery<typeof udfs.latestExport.default>>
>;

function nanosToDate(value: bigint | number): Date {
  if (typeof value === "bigint") {
    return new Date(Number(value / BigInt(1_000_000)));
  }
  return new Date(Math.floor(value / 1_000_000));
}

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
  // Use latestExport.default (the singular, always-present query) rather than
  // .list — the running backend image may be older than the workspace and
  // doesn't have list yet. This shows the most-recent export only.
  const latest = useQuery(udfs.latestExport.default, {});
  const [includeStorage, setIncludeStorage] = useState(true);
  const [isRequesting, setIsRequesting] = useState(false);
  const [requestError, setRequestError] = useState<string | null>(null);
  const includeStorageId = useId();

  const inProgress =
    latest && (latest.state === "requested" || latest.state === "in_progress");
  const buttonBusy = isRequesting || !!inProgress;

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
          <code className="rounded-sm bg-background-tertiary px-1 text-xs">
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
        <Sheet className="flex h-fit w-full shrink-0 flex-col items-start gap-4 xl:w-60">
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
            storage. Manual backups run immediately.
          </p>
          <hr className="w-full" />
          <RestoreFromZipDropzone
            deploymentUrl={deploymentUrl}
            adminKey={adminKey}
          />
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
              // Pass the value through directly: BackupsContent
              // distinguishes `undefined` (still loading) from `null`
              // (loaded — no backups yet). Collapsing them with `??` makes
              // the empty state unreachable and the spinner runs forever.
              latest={latest}
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
  latest,
  deploymentUrl,
  adminKey,
}: {
  // Three-state from `useQuery`: undefined = loading, null = loaded with
  // no rows, value = the latest export. The empty state requires `null`,
  // so we keep all three reachable here.
  latest: ExportRow | null | undefined;
  deploymentUrl: string;
  adminKey: string;
}) {
  if (latest === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center py-10">
        <Spinner />
      </div>
    );
  }

  if (latest === null) {
    return <BackupsEmptyState />;
  }

  // Hide expired completed rows.
  if (
    latest.state === "completed" &&
    Date.now() >= nanosToDate(latest.expiration_ts).getTime()
  ) {
    return <BackupsEmptyState />;
  }

  return (
    <div className="divide-y">
      <BackupRow
        row={latest}
        deploymentUrl={deploymentUrl}
        adminKey={adminKey}
      />
    </div>
  );
}

function BackupRow({
  row,
  deploymentUrl,
  adminKey,
}: {
  row: NonNullable<ExportRow>;
  deploymentUrl: string;
  adminKey: string;
}) {
  const [showDelete, setShowDelete] = useState(false);
  const [showRestore, setShowRestore] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const isPending = row.state === "requested" || row.state === "in_progress";
  const isFailed = row.state === "failed";
  const isCompleted = row.state === "completed";

  const createdAt = new Date(Number(row._creationTime));

  const params = new URLSearchParams({ adminKey });
  const downloadHref = isCompleted
    ? joinUrlPath(
        deploymentUrl,
        `/api/export/zip/${row._id}?${params.toString()}`,
      ).toString()
    : null;
  const filename = `convex-backup-${row._creationTime}.zip`;

  const deleteBackup = async () => {
    setActionError(null);
    try {
      const url = joinUrlPath(
        deploymentUrl,
        `/api/export/set_expiration/${row._id}`,
      ).toString();
      const res = await fetch(url, {
        method: "POST",
        headers: {
          Authorization: `Convex ${adminKey}`,
          "Convex-Client": "dashboard-0.0.0",
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ expirationTsNs: Date.now() * 1_000_000 }),
      });
      if (!res.ok) {
        const body = await res.text();
        throw new Error(`${res.status}: ${body}`);
      }
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
  };

  const cancelBackup = async () => {
    setActionError(null);
    try {
      const url = joinUrlPath(
        deploymentUrl,
        `/api/export/cancel/${row._id}`,
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
      setActionError(e instanceof Error ? e.message : String(e));
    }
  };

  const restoreFromBackup = async () => {
    setActionError(null);
    try {
      const url = joinUrlPath(
        deploymentUrl,
        `/api/export/restore/${row._id}`,
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
      setActionError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex flex-col gap-1 px-4 py-3">
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          {isPending && <Spinner className="size-4" />}
          <div className="flex min-w-0 flex-col">
            <div className="truncate font-mono text-sm">
              {isCompleted ? filename : statusLabel(row.state)}
            </div>
            <div className="text-xs text-content-secondary">
              Created {createdAt.toLocaleString()}
              {isCompleted && (
                <>
                  {" · Expires "}
                  {nanosToDate(row.expiration_ts).toLocaleString()}
                </>
              )}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {isCompleted && downloadHref && (
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
          )}
          <Menu
            placement="bottom-end"
            buttonProps={{
              variant: "neutral",
              size: "xs",
              icon: <DotsVerticalIcon />,
              "aria-label": "Backup options",
            }}
          >
            {isPending ? (
              <MenuItem variant="danger" action={cancelBackup}>
                Cancel
              </MenuItem>
            ) : (
              <>
                {isCompleted && (
                  <MenuItem action={() => setShowRestore(true)}>
                    Restore from this backup
                  </MenuItem>
                )}
                <MenuItem variant="danger" action={() => setShowDelete(true)}>
                  Delete
                </MenuItem>
              </>
            )}
          </Menu>
        </div>
      </div>
      {isFailed && (
        <Callout variant="error" className="mt-1">
          This backup failed. Try again from the &ldquo;Back up now&rdquo;
          button on the left, or report the issue at{" "}
          <Link href="mailto:support@convex.dev">
            <EnvelopeClosedIcon className="mr-0.5 inline" />
            support@convex.dev
          </Link>
          .
        </Callout>
      )}
      {actionError && (
        <Callout variant="error" className="mt-1">
          {actionError}
        </Callout>
      )}
      {showDelete && (
        <ConfirmationDialog
          onClose={() => setShowDelete(false)}
          onConfirm={deleteBackup}
          confirmText="Delete"
          dialogTitle="Delete Backup"
          dialogBody={
            <>
              Are you sure you want to delete this backup from{" "}
              <span className="font-semibold">
                {createdAt.toLocaleString()}
              </span>
              ? The backup zip will no longer be downloadable.
            </>
          }
        />
      )}
      {showRestore && (
        <ConfirmationDialog
          onClose={() => setShowRestore(false)}
          onConfirm={restoreFromBackup}
          confirmText="Restore"
          validationText="restore"
          dialogTitle="Restore from Backup"
          dialogBody={
            <>
              <p className="text-sm">
                The tables in this deployment will be <strong>replaced</strong>{" "}
                by the contents of the backup from{" "}
                <span className="font-semibold">
                  {createdAt.toLocaleString()}
                </span>
                . Existing rows will be deleted before the restore writes the
                backup&apos;s data. The rest of your deployment configuration
                (code, environment variables, scheduled functions) is not
                changed.
              </p>
              <p className="mt-3 text-sm text-content-secondary">
                Type <code className="font-mono">restore</code> to confirm.
              </p>
            </>
          }
        />
      )}
    </div>
  );
}

function statusLabel(state: NonNullable<ExportRow>["state"]): string {
  switch (state) {
    case "requested":
      return "Backup requested";
    case "in_progress":
      return "Backup in progress";
    case "failed":
      return "Backup failed";
    default:
      return state;
  }
}

function RestoreFromZipDropzone({
  deploymentUrl,
  adminKey,
}: {
  deploymentUrl: string;
  adminKey: string;
}) {
  const [dragOver, setDragOver] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [uploadProgress, setUploadProgress] = useState(0);
  const inputId = useId();

  const upload = async (file: File) => {
    if (!file.name.toLowerCase().endsWith(".zip")) {
      setUploadError("Choose a .zip file produced by `npx convex export`.");
      return;
    }
    setUploadError(null);
    setUploading(true);
    setUploadProgress(0);
    try {
      const url = joinUrlPath(
        deploymentUrl,
        "/api/import?tableName=&format=zip&mode=replaceAll",
      ).toString();
      // We use XMLHttpRequest instead of fetch so we can show upload progress
      // — fetch streams aren't widely supported for upload progress yet.
      await new Promise<void>((resolve, reject) => {
        const xhr = new XMLHttpRequest();
        xhr.open("POST", url);
        xhr.setRequestHeader("Authorization", `Convex ${adminKey}`);
        xhr.setRequestHeader("Convex-Client", "dashboard-0.0.0");
        xhr.upload.onprogress = (ev) => {
          if (ev.lengthComputable) {
            setUploadProgress(Math.round((ev.loaded / ev.total) * 100));
          }
        };
        xhr.onerror = () => reject(new Error("network error"));
        xhr.onload = () => {
          if (xhr.status >= 200 && xhr.status < 300) {
            resolve();
          } else {
            reject(new Error(`HTTP ${xhr.status}: ${xhr.responseText}`));
          }
        };
        xhr.send(file);
      });
      // Once the upload completes the deployment kicks off the import; the
      // RestoreStatusBanner (in the larger Backups page) picks it up. Reset
      // local state.
      setUploadProgress(0);
    } catch (err) {
      setUploadError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploading(false);
    }
  };

  return (
    <label
      htmlFor={inputId}
      onDragOver={(e) => {
        e.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={async (e) => {
        e.preventDefault();
        setDragOver(false);
        const file = e.dataTransfer.files?.[0];
        if (file) await upload(file);
      }}
      className={`flex w-full cursor-pointer flex-col items-center gap-1 rounded-md border border-dashed p-3 text-center text-xs text-content-secondary transition-colors ${
        dragOver ? "border-content-primary bg-background-tertiary" : ""
      }`}
    >
      <span>
        <strong className="text-content-primary">
          Restore from local backup
        </strong>
      </span>
      <span>Drop a .zip here or click to browse</span>
      {uploading && (
        <span className="mt-1 font-medium text-content-primary">
          Uploading… {uploadProgress}%
        </span>
      )}
      {uploadError && (
        <span className="mt-1 text-content-error">{uploadError}</span>
      )}
      <input
        id={inputId}
        type="file"
        accept=".zip,application/zip"
        className="sr-only"
        disabled={uploading}
        onChange={async (e) => {
          const file = e.target.files?.[0];
          if (file) await upload(file);
          e.target.value = "";
        }}
      />
    </label>
  );
}

function BackupsEmptyState() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 py-10 text-center">
      <div className="rounded-md bg-purple-100/40 p-2 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300">
        <ArchiveIcon className="size-5" />
      </div>
      <div className="text-content-primary">No backups in this deployment.</div>
      <div className="max-w-sm text-xs text-content-secondary">
        Use the <span className="font-medium">Back up now</span> button on the
        left to create a downloadable snapshot.
      </div>
    </div>
  );
}
