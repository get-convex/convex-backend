import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { useHasProjectAdminPermissions } from "api/roles";
import {
  stringifyValue,
  FunctionNameOption,
  Loading,
  ReadonlyCode,
  useFunctionUrl,
  useNents,
  functionIdentifierValue,
  prettier,
  useCopy,
} from "dashboard-common";
import { ConfirmationDialog } from "elements/ConfirmationDialog";
import { DetailPanel } from "elements/DetailPanel";
import { JSONValue, jsonToConvex } from "convex/values";
import { useCurrentDeployment } from "api/deployments";
import { useCancelJob } from "hooks/deploymentApi";
import Link from "next/link";
import { memo, useState } from "react";
import { ScheduledJob } from "system-udfs/convex/_system/frontend/common";
import { Menu, MenuItem } from "elements/Menu";
import { areEqual } from "react-window";

type JobItemProps = {
  data: { jobs: ScheduledJob[] };
  index: number;
  style: React.CSSProperties;
};
export const JOB_ITEM_SIZE = 50;

export const ScheduledFunctionsListItem = memo(JobItem, areEqual);

function JobItem({ data, index, style }: JobItemProps) {
  const job = data.jobs[index];
  if (!job) {
    return (
      <div style={{ height: JOB_ITEM_SIZE, ...style }}>
        <Loading>
          <div className="h-4" />
        </Loading>
      </div>
    );
  }

  return <JobItemImpl job={job} style={style} />;
}

function JobItemImpl({
  job,
  style,
}: {
  job: ScheduledJob;
  style: React.CSSProperties;
}) {
  const {
    nextTs,
    udfArgs,
    state,
    _id,
    udfPath,
    component: componentPath,
  } = job;
  const { nents } = useNents();
  const componentId = componentPath
    ? (nents?.find((n) => n.path === componentPath)?.id ?? null)
    : null;
  const url = useFunctionUrl(udfPath, componentId);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [showArgs, setShowArgs] = useState(false);
  const { selectedNent } = useNents();
  const cancelJob = useCancelJob();
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canCancelJobs =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;

  if (nextTs === null) {
    throw new Error("Could not find timestamp to run scheduled job at");
  }
  const date = new Date(Number(nextTs / BigInt(1000000))).toLocaleString();
  const udfArgsParsed: JSONValue[] = JSON.parse(
    Buffer.from(udfArgs).toString("utf8"),
  );
  if (_id === null) {
    throw new Error("Scheduled job id is null");
  }

  const currentlyRunning = state.type === "inProgress";
  const copyId = useCopy("Scheduled Function ID");

  return (
    <div style={style} className="border-b transition-all last:border-b-0">
      {showArgs && (
        <DetailPanel
          onClose={() => setShowArgs(false)}
          header="Arguments for scheduled function"
          content={
            <div className="h-full rounded p-4">
              <ReadonlyCode
                disableLineNumbers
                path="scheduling"
                code={`${prettier(`
                          [${udfArgsParsed
                            .map((arg) => stringifyValue(jsonToConvex(arg)))
                            .join(",")}]`).slice(0, -1)} 
                          `}
              />
            </div>
          }
        />
      )}
      <div className="flex items-center gap-4 p-2 text-sm">
        {/* eslint-disable-next-line react/forbid-elements */}
        <button
          type="button"
          className="w-20 truncate text-left text-xs"
          onClick={() => copyId(_id)}
        >
          {_id.substring(0, 10)}...
        </button>
        <span className="w-36 text-xs">{date}</span>
        <span className="w-20 text-xs text-content-secondary">
          {currentlyRunning ? "Running" : "Pending"}
        </span>
        <div className="w-48 text-xs hover:underline">
          <Link href={url}>
            <FunctionNameOption
              label={functionIdentifierValue(
                udfPath,
                componentPath,
                componentId ?? undefined,
              )}
              oneLine
            />
          </Link>
        </div>
        <div className="ml-auto">
          <Menu
            placement="bottom-end"
            buttonProps={{
              "aria-label": "Scheduled function settings",
              icon: <DotsVerticalIcon />,
              size: "xs",
              variant: "neutral",
            }}
          >
            <MenuItem action={() => setShowArgs(true)}>View Arguments</MenuItem>
            <MenuItem
              action={() => setShowDeleteModal(true)}
              disabled={currentlyRunning || !canCancelJobs}
              tip={
                !canCancelJobs &&
                "You do not have permission to cancel scheduled runs in production."
              }
              variant="danger"
            >
              Cancel
            </MenuItem>
          </Menu>
        </div>
      </div>
      <div>
        {showDeleteModal && (
          <ConfirmationDialog
            onClose={() => setShowDeleteModal(false)}
            onConfirm={() => cancelJob(_id, selectedNent?.id ?? null)}
            confirmText="Confirm"
            dialogTitle="Cancel run"
            dialogBody={
              <div>You are canceling a run scheduled to occur at {date}.</div>
            }
          />
        )}
      </div>
    </div>
  );
}
