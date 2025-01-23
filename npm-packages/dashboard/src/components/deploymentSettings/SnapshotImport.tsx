import {
  CheckCircledIcon,
  CheckIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  CrossCircledIcon,
} from "@radix-ui/react-icons";
import {
  Spinner,
  TimestampDistance,
  Tooltip,
  Button,
  PuzzlePieceIcon,
  Sheet,
  ConfirmationDialog,
} from "dashboard-common";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { Doc, Id } from "system-udfs/convex/_generated/dataModel";
import { formatDistanceStrict } from "date-fns";
import Link from "next/link";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { useCurrentTeam, useTeamMembers } from "api/teams";
import { snapshotImportFormat } from "system-udfs/convex/tableDefs/snapshotImport";
import { Infer } from "convex/values";
import { useCancelImport, useConfirmImport } from "hooks/deploymentApi";
import { Disclosure } from "@headlessui/react";
import { useState } from "react";

function ConfirmImportButton({
  snapshotImport,
}: {
  snapshotImport: Doc<"_snapshot_imports">;
}) {
  const [showDialog, setShowDialog] = useState(false);
  const confirmImport = useConfirmImport();
  return (
    <>
      {showDialog && (
        <ConfirmationDialog
          confirmText="Confirm"
          dialogTitle="Confirm import"
          variant="primary"
          dialogBody={
            <div>
              <div>Are you sure you want to perform the following import?</div>
              <ImportSummary
                snapshotImportCheckpoints={snapshotImport.checkpoints}
              />
            </div>
          }
          onConfirm={() => confirmImport(snapshotImport._id)}
          onClose={() => setShowDialog(false)}
        />
      )}
      <Button onClick={() => setShowDialog(true)}>Confirm</Button>
    </>
  );
}

function CancelImportButton({
  importId,
}: {
  importId: Id<"_snapshot_imports">;
}) {
  const [showDialog, setShowDialog] = useState(false);
  const cancelImport = useCancelImport();
  return (
    <>
      {showDialog && (
        <ConfirmationDialog
          confirmText="Confirm"
          dialogTitle="Cancel import"
          dialogBody="Are you sure you want to cancel this import?"
          onConfirm={() => cancelImport(importId)}
          onClose={() => setShowDialog(false)}
        />
      )}
      <Button variant="danger" onClick={() => setShowDialog(true)}>
        Cancel
      </Button>
    </>
  );
}

function ImportStateBody({
  snapshotImport,
}: {
  snapshotImport: Doc<"_snapshot_imports">;
}) {
  switch (snapshotImport.state.state) {
    case "uploaded":
      return (
        <div className="flex items-center gap-2">
          <Spinner className="ml-0" /> Uploading snapshot
        </div>
      );
    case "waiting_for_confirmation":
      return (
        <div>
          {(snapshotImport.checkpoints === null ||
            snapshotImport.checkpoints === undefined) && (
            <div className="whitespace-pre-wrap font-mono">
              {snapshotImport.state.message_to_confirm}
            </div>
          )}
          <div className="flex gap-2">
            <ConfirmImportButton snapshotImport={snapshotImport} />
            <CancelImportButton importId={snapshotImport._id} />
          </div>
        </div>
      );
    case "in_progress":
      return (
        <div className="flex flex-col">
          {snapshotImport.state.checkpoint_messages.map((message: string) => (
            <div className="flex items-center gap-2">
              <CheckIcon /> {message}
            </div>
          ))}
          <div className="flex items-center gap-2">
            <Spinner className="ml-0" /> {snapshotImport.state.progress_message}
          </div>
        </div>
      );
    case "completed": {
      const completedDate = new Date(
        Number(snapshotImport.state.timestamp / BigInt(1_000_000)),
      );
      return (
        <div>
          <Tooltip tip={completedDate.toLocaleString()}>
            <div className="flex items-center gap-1 border p-1 text-sm text-content-primary">
              <CheckCircledIcon className="min-w-[1rem] text-util-success" />
              {`Completed ${formatDistanceStrict(completedDate, new Date(), {
                addSuffix: true,
              })}`}
            </div>
          </Tooltip>
        </div>
      );
    }
    case "failed":
      return (
        <div className="flex w-fit items-center gap-1 rounded border p-1 text-sm">
          <CrossCircledIcon className="min-w-[1rem] text-content-errorSecondary" />
          {snapshotImport.state.error_message}
        </div>
      );

    default:
      throw new Error(
        `unexpected snapshot import state ${snapshotImport.state}`,
      );
  }
}

function ImportStatePill({
  snapshotImportState,
}: {
  snapshotImportState: Doc<"_snapshot_imports">["state"]["state"];
}) {
  switch (snapshotImportState) {
    case "uploaded":
    case "waiting_for_confirmation":
      return (
        <span className="h-fit w-fit rounded bg-blue-100 p-1 text-center text-xs text-blue-900 dark:bg-blue-900 dark:text-blue-100">
          pending confirmation
        </span>
      );
    case "in_progress":
      return (
        <span className="h-fit w-fit rounded bg-blue-100 p-1 text-center text-xs text-blue-900 dark:bg-blue-900 dark:text-blue-100">
          in progress
        </span>
      );

    case "completed":
      return (
        <span className="h-fit w-14 rounded bg-background-success p-1 text-center text-xs text-content-success">
          success
        </span>
      );
    case "failed":
      return (
        <span className="h-fit w-14 rounded bg-background-error p-1 text-center text-xs text-content-error">
          failure
        </span>
      );
    default:
      throw new Error(
        `unexpected snapshot import state ${snapshotImportState}`,
      );
  }
}

function snapshotImportFormatToText(
  format: Infer<typeof snapshotImportFormat>,
) {
  switch (format.format) {
    case "csv":
      return "CSV";
    case "jsonl":
      return "JSONL";
    case "json_array":
      return "JSON";
    case "zip":
      return "ZIP";
    default: {
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const _: never = format;
      return "";
    }
  }
}

export function ImportSummary({
  snapshotImportCheckpoints,
}: {
  snapshotImportCheckpoints:
    | Doc<"_snapshot_imports">["checkpoints"]
    | null
    | undefined;
}) {
  if (
    snapshotImportCheckpoints === null ||
    snapshotImportCheckpoints === undefined
  ) {
    return null;
  }
  const checkpointsByComponent = snapshotImportCheckpoints.reduce(
    (acc, checkpoint) => {
      const componentPath = checkpoint.component_path ?? undefined;
      if (!acc.has(componentPath)) {
        acc.set(componentPath, []);
      }
      acc.get(componentPath)?.push(checkpoint);
      return acc;
    },
    new Map<
      string | undefined,
      NonNullable<Doc<"_snapshot_imports">["checkpoints"]>
    >(),
  );
  return (
    <div>
      {Array.from(checkpointsByComponent.entries()).map(
        ([componentPath, checkpoints]) => (
          <div key={componentPath}>
            {componentPath ? (
              <div className="flex w-full items-center space-x-1">
                <PuzzlePieceIcon />
                <span>{componentPath}</span>
              </div>
            ) : null}
            <table className="mr-auto border-collapse border text-left">
              <thead className="border">
                <tr>
                  <th className="border px-2 font-semibold">table</th>
                  <th className="border px-2 font-semibold">create</th>
                  <th className="border px-2 font-semibold">delete</th>
                </tr>
              </thead>
              <tbody>
                {checkpoints.map((checkpoint) => (
                  <tr key={checkpoint.display_table_name}>
                    <td className="border px-2">
                      <span>{checkpoint.display_table_name}</span>
                    </td>
                    <td className="border px-2">
                      {Number(
                        checkpoint.total_num_rows_to_write,
                      ).toLocaleString()}{" "}
                      {checkpoint.display_table_name === "_storage"
                        ? `file${Number(checkpoint.total_num_rows_to_write) === 1 ? "" : "s"}`
                        : `document${Number(checkpoint.total_num_rows_to_write) === 1 ? "" : "s"}`}
                    </td>
                    <td className="border px-2">{`${Number(checkpoint.existing_rows_to_delete).toLocaleString()} of ${Number(checkpoint.existing_rows_in_table).toLocaleString()} ${checkpoint.display_table_name === "_storage" ? "files" : "documents"}`}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ),
      )}
    </div>
  );
}

function ImportState({
  snapshotImport,
}: {
  snapshotImport: Doc<"_snapshot_imports"> & { memberName: string };
}) {
  return (
    <Disclosure>
      {({ open }) => (
        <>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex gap-2">
              <ImportStatePill
                snapshotImportState={snapshotImport.state.state}
              />
              <div className="flex flex-wrap items-center gap-1">
                <TeamMemberLink
                  memberId={
                    snapshotImport.member_id === null ||
                    snapshotImport.member_id === undefined
                      ? snapshotImport.member_id
                      : Number(snapshotImport.member_id)
                  }
                  name={snapshotImport.memberName}
                />

                {`imported a snapshot from a ${snapshotImportFormatToText(snapshotImport.format)} file.`}
              </div>
            </div>
            <div className="flex items-center gap-2">
              <TimestampDistance
                prefix="Started "
                date={new Date(snapshotImport._creationTime)}
              />
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
            </div>
          </div>
          <Disclosure.Panel>
            <div className="mt-2 flex flex-col gap-2">
              <ImportStateBody snapshotImport={snapshotImport} />
              <ImportSummary
                snapshotImportCheckpoints={snapshotImport.checkpoints}
              />
            </div>
          </Disclosure.Panel>
        </>
      )}
    </Disclosure>
  );
}

const useSnapshotImports = () => {
  const existingImports: Doc<"_snapshot_imports">[] | undefined =
    useQuery(udfs.snapshotImport.list) ?? [];
  const currentTeam = useCurrentTeam();
  const teamMembers = useTeamMembers(currentTeam?.id) ?? [];
  return existingImports
    ?.filter((s) => s.requestor.type === "snapshotImport")
    .map((s) => {
      const member = teamMembers.find((m) => BigInt(m.id) === s.member_id);
      const memberName = member?.name || member?.email || "Unknown member";
      return {
        ...s,
        memberName,
      };
    });
};

export function SnapshotImport() {
  const existingImports = useSnapshotImports();

  return (
    <Sheet>
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-4">
          <div>
            <h3 className="mb-2">Snapshot Import</h3>
            <p className="text-content-primary">
              Import tables into your database from a snapshot.{" "}
              <Link
                target="_blank"
                href="https://docs.convex.dev/database/import-export/import"
                className="text-content-link hover:underline dark:underline"
              >
                Learn more
              </Link>
            </p>
          </div>
        </div>
        {existingImports && existingImports.length !== 0 && (
          <div className="flex flex-col gap-4">
            <h4>Recent Imports</h4>
            {existingImports.map(
              (
                existingImport: Doc<"_snapshot_imports"> & {
                  memberName: string;
                },
              ) => (
                <div className="text-content-primary" key={existingImport._id}>
                  <ImportState snapshotImport={existingImport} />
                </div>
              ),
            )}
          </div>
        )}
      </div>
    </Sheet>
  );
}
