import { TrashIcon } from "@radix-ui/react-icons";
import { useRouter } from "next/router";
import { useContext, useState } from "react";
import { ScheduledJob } from "system-udfs/convex/_system/frontend/common";
import { useCancelAllJobs } from "@common/features/schedules/lib/api";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import {
  itemIdentifier,
  useCurrentOpenFunction,
  useModuleFunctions,
} from "@common/lib/functions/FunctionsProvider";
import { Combobox } from "@common/elements/Combobox";
import {
  displayName,
  functionIdentifierFromValue,
  functionIdentifierValue,
} from "@common/lib/functions/generateFileTree";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { Button } from "@common/elements/Button";
import { SchedulerStatus } from "@common/elements/SchedulerStatus";
import { ConfirmationDialog } from "@common/elements/ConfirmationDialog";

export function ScheduledFunctionsContentToolbar({
  jobs,
}: {
  jobs: ScheduledJob[];
}) {
  const currentOpenFunction = useCurrentOpenFunction();
  const moduleFunctions = useModuleFunctions();
  const router = useRouter();
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const cancelJobs = useCancelAllJobs();

  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canCancelJobs =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;

  return (
    <div className="flex w-full flex-wrap items-center gap-4">
      <div className="flex w-full flex-wrap gap-4">
        <Combobox
          label="Filter scheduled runs by function"
          className="w-[22rem]"
          buttonClasses="w-[22rem]"
          optionsWidth="full"
          searchPlaceholder="Search functions..."
          selectedOption={
            currentOpenFunction ? itemIdentifier(currentOpenFunction) : null
          }
          setSelectedOption={async (item) => {
            if (!item) {
              const { function: _, ...query } = router.query;
              await router.push({ query });
              return;
            }
            const { identifier, componentPath } =
              functionIdentifierFromValue(item);
            await router.push({
              query: {
                ...router.query,
                function: identifier,
                componentPath,
              },
            });
          }}
          options={[
            {
              label: functionIdentifierValue("All functions"),
              value: null,
            },
            ...moduleFunctions.map((value) => ({
              label: itemIdentifier(value),
              value: itemIdentifier(value),
            })),
          ]}
          Option={(props) => (
            <FunctionNameOption {...{ ...props, disableTruncation: true }} />
          )}
          processFilterOption={(option) => {
            const id = functionIdentifierFromValue(option);
            return id.componentPath
              ? `${id.componentPath}/${id.identifier}`
              : id.identifier;
          }}
        />
        <Button
          variant="danger"
          size="sm"
          onClick={() => setShowDeleteModal(true)}
          icon={<TrashIcon />}
          disabled={jobs.length === 0 || !canCancelJobs}
          tip={
            !canCancelJobs &&
            "You do not have permission to cancel scheduled runs in production."
          }
        >
          Cancel All
        </Button>

        <div className="ml-auto">
          <SchedulerStatus small />
        </div>
      </div>
      {showDeleteModal && (
        <ConfirmationDialog
          onClose={() => setShowDeleteModal(false)}
          onConfirm={() => cancelJobs(currentOpenFunction?.identifier)}
          confirmText="Confirm"
          dialogTitle="Cancel all runs"
          validationText={
            deployment?.deploymentType === "prod" ? "Cancel all" : undefined
          }
          dialogBody={
            <div>
              You are canceling all scheduled runs for{" "}
              {currentOpenFunction?.displayName ? (
                <span className="font-mono font-semibold">
                  {displayName(
                    currentOpenFunction?.displayName,
                    currentOpenFunction?.componentPath ?? null,
                  )}
                </span>
              ) : (
                "all functions"
              )}
              .
            </div>
          }
        />
      )}
    </div>
  );
}
