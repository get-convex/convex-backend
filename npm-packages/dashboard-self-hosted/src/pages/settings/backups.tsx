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
  CheckCircledIcon,
  CrossCircledIcon,
  DotsVerticalIcon,
  DownloadIcon,
  EnvelopeClosedIcon,
} from "@radix-ui/react-icons";
import { useQuery } from "convex/react";
import { useContext, useEffect, useId, useRef, useState } from "react";

type ExportRow = NonNullable<
  ReturnType<typeof useQuery<typeof udfs.latestExport.list>>
>[number];

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
  const exports = useQuery(udfs.latestExport.list, {});
  const [includeStorage, setIncludeStorage] = useState(true);
  const [isRequesting, setIsRequesting] = useState(false);
  const [requestError, setRequestError] = useState<string | null>(null);
  const includeStorageId = useId();

  const inProgress = exports?.some(
    (e) => e.state === "requested" || e.state === "in_progress",
  );
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
          <RestoreStatusBanner
            deploymentUrl={deploymentUrl}
            adminKey={adminKey}
          />
          <Sheet padding={false} className="flex min-h-72 flex-col">
            <div className="flex items-center justify-between border-b px-4 py-3">
              <h4>Existing Backups</h4>
            </div>
            <BackupsContent
              exports={exports}
              deploymentUrl={deploymentUrl}
              adminKey={adminKey}
            />
          </Sheet>
        </div>
      </div>
    </div>
  );
}

/**
 * Banner that surfaces the most recent in-flight or recently-finished
 * snapshot import. Useful for users running `npx convex import` from the CLI
 * — the dashboard shows the same progress / outcome they'd see if they were
 * waiting in the terminal.
 */
function RestoreStatusBanner({
  deploymentUrl,
  adminKey,
}: {
  deploymentUrl: string;
  adminKey: string;
}) {
  const imports = useQuery(udfs.snapshotImport.list);
  const autoConfirmedRef = useRef<Set<string>>(new Set());

  // When an import we triggered server-side reaches `WaitingForConfirmation`,
  // auto-confirm it by POSTing /api/perform_import. Mirrors what the cloud
  // BackupRestoreStatus useEffect does, so the user doesn't have to click a
  // separate "confirm" button before the restore can run.
  const latest = imports && imports.length > 0 ? imports[0] : null;
  const latestId = latest?._id;
  const latestState = latest?.state.state;
  useEffect(() => {
    if (!latestId || latestState !== "waiting_for_confirmation") return;
    if (autoConfirmedRef.current.has(latestId)) return;
    autoConfirmedRef.current.add(latestId);
    void (async () => {
      try {
        const url = joinUrlPath(
          deploymentUrl,
          `/api/perform_import`,
        ).toString();
        await fetch(url, {
          method: "POST",
          headers: {
            Authorization: `Convex ${adminKey}`,
            "Convex-Client": "dashboard-0.0.0",
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ importId: latestId }),
        });
      } catch {
        // Surfaced via the existing failed-state banner if the worker errors.
      }
    })();
  }, [latestId, latestState, deploymentUrl, adminKey]);

  if (!imports || imports.length === 0 || !latest) return null;
  const state = latest.state;

  if (state.state === "in_progress") {
    return (
      <RestoreOngoing
        title="Restoring from a backup"
        message={state.progress_message ?? "Restoring snapshot…"}
      />
    );
  }
  if (
    state.state === "uploaded" ||
    state.state === "waiting_for_confirmation"
  ) {
    return (
      <RestoreOngoing
        title="Restoring from a backup"
        message="Starting the restore…"
      />
    );
  }
  if (state.state === "completed") {
    const completedAt = new Date(Number(state.timestamp / BigInt(1_000_000)));
    // Don't render forever — stale "completed" banners would be noise.
    const ageMs = Date.now() - completedAt.getTime();
    const oneHour = 60 * 60 * 1000;
    if (ageMs > oneHour) return null;
    const rows = Number(state.num_rows_written);
    return (
      <Callout className="flex items-center gap-2">
        <CheckCircledIcon className="size-4 shrink-0 text-content-success" />
        <span className="text-sm">
          Restored <strong>{rows.toLocaleString()}</strong>{" "}
          {rows === 1 ? "document" : "documents"} from a backup{" "}
          {timeAgo(completedAt)}.
        </span>
      </Callout>
    );
  }
  if (state.state === "failed") {
    return (
      <Callout variant="error" className="flex items-start gap-2">
        <CrossCircledIcon className="size-4 shrink-0 text-content-errorSecondary" />
        <div className="flex flex-col gap-1">
          <span className="text-sm">
            The most recent restore failed{" "}
            {timeAgo(new Date(latest._creationTime))}.
          </span>
          <code className="text-xs whitespace-pre-wrap">
            {state.error_message}
          </code>
        </div>
      </Callout>
    );
  }
  return null;
}

function RestoreOngoing({
  title,
  message,
}: {
  title: string;
  message: string;
}) {
  return (
    <div className="flex min-h-16 flex-col flex-wrap justify-center gap-2 rounded-lg border bg-background-secondary px-4 py-2 text-sm">
      <div className="flex flex-wrap justify-end gap-4">
        <div className="grow font-semibold">{title}</div>
        <div className="min-w-56 text-right text-content-secondary">
          {message}
        </div>
      </div>
      <div className="h-1 w-full overflow-hidden rounded bg-background-tertiary">
        <div className="h-full w-1/3 animate-pulse bg-content-link" />
      </div>
    </div>
  );
}

function timeAgo(date: Date): string {
  const ms = Date.now() - date.getTime();
  const sec = Math.round(ms / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.round(hr / 24)}d ago`;
}

function BackupsContent({
  exports,
  deploymentUrl,
  adminKey,
}: {
  exports: ExportRow[] | undefined;
  deploymentUrl: string;
  adminKey: string;
}) {
  if (exports === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center py-10">
        <Spinner />
      </div>
    );
  }

  // Hide expired completed rows from the list — they can't be downloaded any
  // more and just clutter the UI.
  const visible = exports.filter((e) => {
    if (e.state !== "completed") return true;
    return Date.now() < Number(e.expiration_ts / BigInt(1_000_000));
  });

  if (visible.length === 0) {
    return <BackupsEmptyState message="No backups in this deployment." />;
  }

  return (
    <div className="divide-y">
      {visible.map((row) => (
        <BackupRow
          key={row._id}
          row={row}
          deploymentUrl={deploymentUrl}
          adminKey={adminKey}
        />
      ))}
    </div>
  );
}

function BackupRow({
  row,
  deploymentUrl,
  adminKey,
}: {
  row: ExportRow;
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
        // Setting expiration to "now" tells the backend the export is no
        // longer downloadable; the next GC pass cleans it up.
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
                  {new Date(
                    Number(row.expiration_ts / BigInt(1_000_000)),
                  ).toLocaleString()}
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

function statusLabel(state: ExportRow["state"]): string {
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
