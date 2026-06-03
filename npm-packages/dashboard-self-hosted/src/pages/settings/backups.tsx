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
import { Combobox } from "@ui/Combobox";
import { cn } from "@ui/cn";
import {
  ArchiveIcon,
  CheckCircledIcon,
  ClockIcon,
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

/**
 * Convert a Convex int64 timestamp (nanoseconds since the unix epoch) into a
 * `Date`. Convex's `v.int64()` always reaches the client as `bigint`, but we
 * also tolerate plain numbers in case the wire format ever shifts — runtime
 * `bigint -> number` arithmetic can throw 'cannot convert BigInt to number'
 * if mixed implicitly, so this helper is the single place we narrow the type.
 */
function nanosToDate(value: bigint | number): Date {
  if (typeof value === "bigint") {
    // BigInt(1_000_000) avoids the ES2020 1_000_000n literal that the
    // dashboard's tsconfig target doesn't allow.
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
          <PeriodicBackupSelector
            deploymentUrl={deploymentUrl}
            adminKey={adminKey}
          />
          <hr className="w-full" />
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
            storage. Manual backups run immediately; periodic backups fire in
            the background per the schedule above.
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
    const completedAt = nanosToDate(state.timestamp);
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
      <div className="h-1 w-full overflow-hidden rounded-sm bg-background-tertiary">
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
    return Date.now() < nanosToDate(e.expiration_ts).getTime();
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

/**
 * UI for the singleton `_periodic_backup_config` row. Reads the current
 * config via `udfs.periodicBackup.get` (system query) and writes via
 * POST /api/periodic_backup/configure or /api/periodic_backup/disable.
 *
 * Picker is intentionally restricted to a small set of common cadences
 * (hourly / daily / weekly) — anything beyond that is best expressed as
 * a hand-written cronspec and configured directly via the HTTP endpoint.
 */
function PeriodicBackupSelector({
  deploymentUrl,
  adminKey,
}: {
  deploymentUrl: string;
  adminKey: string;
}) {
  const config = useQuery(udfs.periodicBackup.get, {});
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const checkboxId = useId();

  // Local edit state mirrors the persisted config but lets the user adjust
  // the cron picker without immediately writing on every keystroke.
  const [enabled, setEnabled] = useState(false);
  const [frequency, setFrequency] = useState<"hourly" | "daily" | "weekly">(
    "daily",
  );
  // Hour and dayOfWeek are stored in the user's *local* timezone — we convert
  // to UTC at the cron boundary in buildCronspec.
  const [hour, setHour] = useState(3);
  const [dayOfWeek, setDayOfWeek] = useState(0); // Sunday (local)
  const [includeStorage, setIncludeStorage] = useState(false);
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    if (config === undefined || hydrated) return;
    if (config === null) {
      setEnabled(false);
    } else {
      setEnabled(true);
      // Defensive: coerce in case the persisted row was ever written by
      // an older serialization that didn't include the field.
      setIncludeStorage(Boolean(config.include_storage));
      const parsed = parseCronspec(config.cronspec);
      if (parsed) {
        setFrequency(parsed.frequency);
        if (
          parsed.frequency === "weekly" &&
          parsed.hourUtc !== undefined &&
          parsed.dayOfWeek !== undefined
        ) {
          const local = utcToLocal(parsed.hourUtc, parsed.dayOfWeek);
          setHour(local.hour);
          setDayOfWeek(local.dayOfWeek);
        } else if (
          parsed.frequency === "daily" &&
          parsed.hourUtc !== undefined
        ) {
          setHour(utcToLocal(parsed.hourUtc).hour);
        }
      }
    }
    setHydrated(true);
  }, [config, hydrated]);

  const submit = async (newEnabled: boolean) => {
    setSubmitting(true);
    setError(null);
    try {
      if (!newEnabled) {
        const url = joinUrlPath(
          deploymentUrl,
          `/api/periodic_backup/disable`,
        ).toString();
        const res = await fetch(url, {
          method: "POST",
          headers: {
            Authorization: `Convex ${adminKey}`,
            "Convex-Client": "dashboard-0.0.0",
          },
        });
        if (!res.ok) throw new Error(`${res.status}: ${await res.text()}`);
      } else {
        const cronspec = buildCronspec(frequency, hour, dayOfWeek);
        const url = joinUrlPath(
          deploymentUrl,
          `/api/periodic_backup/configure`,
        ).toString();
        const res = await fetch(url, {
          method: "POST",
          headers: {
            Authorization: `Convex ${adminKey}`,
            "Convex-Client": "dashboard-0.0.0",
            "Content-Type": "application/json",
          },
          // Coerce to a definite boolean so we never accidentally drop the
          // field if local state was never written (JSON.stringify omits
          // `undefined` values and the backend would 400 on the missing
          // `includeStorage`).
          body: JSON.stringify({
            cronspec,
            includeStorage: Boolean(includeStorage),
          }),
        });
        if (!res.ok) throw new Error(`${res.status}: ${await res.text()}`);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  if (config === undefined) {
    return (
      <div className="flex w-full items-center justify-center py-2">
        <Spinner className="size-4" />
      </div>
    );
  }

  // Dirty-state detection: only show the Save button when the local edits
  // diverge from what's persisted. Avoids the "always-on" Save that begs
  // the question "did I change anything?".
  const dirty = (() => {
    if (!enabled) return false;
    if (config === null) return true; // first time turning on
    const persistedParsed = parseCronspec(config.cronspec);
    if (!persistedParsed) return true; // we couldn't read it; safer to allow save
    if (persistedParsed.frequency !== frequency) return true;
    if (Boolean(config.include_storage) !== includeStorage) return true;
    if (frequency === "hourly") return false;
    // Compare against persisted UTC values by re-projecting the local edits
    // through the same conversion buildCronspec uses.
    const editedUtc = localToUtc(
      hour,
      frequency === "weekly" ? dayOfWeek : undefined,
    );
    if (persistedParsed.hourUtc !== editedUtc.hour) return true;
    if (
      frequency === "weekly" &&
      persistedParsed.dayOfWeek !== editedUtc.dayOfWeek
    )
      return true;
    return false;
  })();

  return (
    <div className="flex w-full flex-col gap-3">
      <label
        htmlFor={checkboxId}
        className="flex items-center gap-2 text-sm text-content-primary"
      >
        <Checkbox
          id={checkboxId}
          checked={enabled}
          disabled={submitting}
          onChange={async () => {
            const next = !enabled;
            setEnabled(next);
            await submit(next);
          }}
        />
        <span className="font-medium">Backup automatically</span>
        {submitting && <Spinner className="size-3" />}
      </label>
      {enabled && (
        <div className="flex flex-col gap-3">
          <FieldLabel>Frequency</FieldLabel>
          <div
            role="radiogroup"
            aria-label="Frequency"
            className="flex w-full overflow-hidden rounded-md border"
          >
            {(
              [
                { label: "Hourly", value: "hourly" },
                { label: "Daily", value: "daily" },
                { label: "Weekly", value: "weekly" },
              ] as const
            ).map((option, i) => {
              const selected = frequency === option.value;
              return (
                // eslint-disable-next-line react/forbid-elements -- @ui/Button doesn't support the radio role / aria-checked semantics this segmented row needs.
                <button
                  key={option.value}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  disabled={submitting}
                  onClick={() => setFrequency(option.value)}
                  className={cn(
                    "flex-1 px-2 py-1 text-xs font-medium transition-colors",
                    "focus-visible:z-10 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-border-selected",
                    "disabled:cursor-not-allowed disabled:opacity-50",
                    i > 0 && "border-l",
                    selected
                      ? "bg-background-secondary text-content-primary"
                      : "bg-background-tertiary/40 text-content-secondary hover:bg-background-tertiary",
                  )}
                >
                  {option.label}
                </button>
              );
            })}
          </div>

          {frequency === "weekly" && (
            <div className="flex flex-col gap-1">
              <FieldLabel>Day of week</FieldLabel>
              <Combobox
                label="Day of week"
                options={[
                  "Sunday",
                  "Monday",
                  "Tuesday",
                  "Wednesday",
                  "Thursday",
                  "Friday",
                  "Saturday",
                ].map((name, value) => ({ label: name, value }))}
                selectedOption={dayOfWeek}
                setSelectedOption={(v) => v !== null && setDayOfWeek(v)}
                buttonClasses="w-full"
                disableSearch
                disabled={submitting}
                size="sm"
              />
            </div>
          )}

          {frequency !== "hourly" && (
            <div className="flex flex-col gap-1">
              <FieldLabel>Time ({localTimezoneName()})</FieldLabel>
              <Combobox
                label="Time"
                options={Array.from({ length: 24 }, (_, h) => ({
                  label: `${String(h).padStart(2, "0")}:00`,
                  value: h,
                }))}
                selectedOption={hour}
                setSelectedOption={(v) => v !== null && setHour(v)}
                buttonClasses="w-full"
                disabled={submitting}
                size="sm"
              />
            </div>
          )}

          <label className="mt-1 flex items-center gap-2 text-xs text-content-primary">
            <Checkbox
              checked={includeStorage}
              disabled={submitting}
              onChange={() => setIncludeStorage((v) => !v)}
            />
            <span>Include file storage</span>
          </label>

          {(config?.next_run_ts !== undefined || config?.last_run_ts) && (
            <div className="mt-1 flex items-start gap-2 border-t pt-3 text-[11px] text-content-secondary">
              <ClockIcon className="mt-px size-3 shrink-0" />
              <div className="flex min-w-0 flex-col gap-0.5">
                {config?.next_run_ts !== undefined && (
                  <div>
                    <span className="text-content-tertiary">Next run </span>
                    <span className="text-content-primary">
                      {nanosToDate(config.next_run_ts).toLocaleString(
                        undefined,
                        {
                          month: "short",
                          day: "numeric",
                          hour: "2-digit",
                          minute: "2-digit",
                        },
                      )}
                    </span>
                  </div>
                )}
                {config?.last_run_ts !== undefined &&
                  config.last_run_ts !== null && (
                    <div>
                      <span className="text-content-tertiary">Last run </span>
                      <span className="text-content-primary">
                        {nanosToDate(config.last_run_ts).toLocaleString(
                          undefined,
                          {
                            month: "short",
                            day: "numeric",
                            hour: "2-digit",
                            minute: "2-digit",
                          },
                        )}
                      </span>
                    </div>
                  )}
              </div>
            </div>
          )}

          {dirty && (
            <div className="flex justify-end">
              <Button
                size="xs"
                disabled={submitting}
                loading={submitting}
                onClick={() => submit(true)}
              >
                Save changes
              </Button>
            </div>
          )}
        </div>
      )}
      {error && (
        <Callout variant="error" className="text-xs">
          {error}
        </Callout>
      )}
    </div>
  );
}

/**
 * Tiny uppercase micro-label used above each picker control. Distinct from
 * the rest of the dashboard's label patterns on purpose — the schedule card
 * is a settings sub-form and benefits from the extra typographic step-down.
 */
function FieldLabel({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-[10px] font-medium tracking-wider text-content-tertiary uppercase">
      {children}
    </span>
  );
}

/**
 * Build a 5-field UTC cron expression from the picker's choices, which are
 * expressed in the user's local timezone. For weekly schedules we have to
 * convert *both* hour and day-of-week together — picking "Sunday 23:00" in
 * UTC-5 maps to "Monday 04:00 UTC", not "Sunday 04:00 UTC".
 */
function buildCronspec(
  frequency: "hourly" | "daily" | "weekly",
  hourLocal: number,
  dayOfWeekLocal: number,
): string {
  switch (frequency) {
    case "hourly":
      return `0 * * * *`;
    case "daily": {
      const { hour } = localToUtc(hourLocal);
      return `0 ${hour} * * *`;
    }
    case "weekly": {
      const { hour, dayOfWeek } = localToUtc(hourLocal, dayOfWeekLocal);
      return `0 ${hour} * * ${dayOfWeek}`;
    }
  }
}

/**
 * Convert a (dayOfWeek, hour) wall-clock time in the user's local timezone
 * to the equivalent (dayOfWeek, hour) in UTC. Anchored at a known Sunday so
 * the day-of-week math wraps cleanly across midnight.
 */
function localToUtc(
  hourLocal: number,
  dayOfWeekLocal?: number,
): { hour: number; dayOfWeek: number } {
  // 2024-01-07 was a Sunday.
  const d = new Date(2024, 0, 7 + (dayOfWeekLocal ?? 0), hourLocal, 0);
  return { hour: d.getUTCHours(), dayOfWeek: d.getUTCDay() };
}

/** Inverse of `localToUtc`. */
function utcToLocal(
  hourUtc: number,
  dayOfWeekUtc?: number,
): { hour: number; dayOfWeek: number } {
  const d = new Date(Date.UTC(2024, 0, 7 + (dayOfWeekUtc ?? 0), hourUtc, 0));
  return { hour: d.getHours(), dayOfWeek: d.getDay() };
}

/**
 * Short localized name of the user's timezone (e.g. "PST", "GMT+9"). Used
 * to label the time picker so the user knows which zone the picker is in.
 */
function localTimezoneName(): string {
  const part = new Intl.DateTimeFormat(undefined, {
    timeZoneName: "short",
  })
    .formatToParts(new Date())
    .find((p) => p.type === "timeZoneName");
  return part?.value ?? "local";
}

/**
 * Best-effort reverse of `buildCronspec` so the picker hydrates from a
 * persisted config. Anything we don't recognize falls back to "daily 03:00"
 * and the user can re-pick from there.
 */
function parseCronspec(cronspec: string): {
  frequency: "hourly" | "daily" | "weekly";
  hourUtc?: number;
  dayOfWeek?: number;
} | null {
  const parts = cronspec.trim().split(/\s+/);
  if (parts.length !== 5) return null;
  const [minute, hour, , , dow] = parts;
  if (minute !== "0") return null;
  if (hour === "*") return { frequency: "hourly" };
  const h = Number(hour);
  if (!Number.isInteger(h) || h < 0 || h > 23) return null;
  if (dow === "*") return { frequency: "daily", hourUtc: h };
  const d = Number(dow);
  if (!Number.isInteger(d) || d < 0 || d > 6) return null;
  return { frequency: "weekly", hourUtc: h, dayOfWeek: d };
}
