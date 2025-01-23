import { PlayIcon } from "@radix-ui/react-icons";
import {
  ProductionEditsConfirmationDialog,
  Button,
  Loading,
  ModuleFunction,
  useShowGlobalRunner,
} from "dashboard-common";
import { FunctionRunnerDisabledWhilePaused } from "components/functions/FunctionRunnerDisabledWhilePaused";
import { useQuery } from "convex/react";
import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import { useIsDeploymentPaused } from "hooks/useIsDeploymentPaused";
import classNames from "classnames";
import { useState } from "react";
import { useSessionStorage } from "react-use";
import { lt } from "semver";
import udfs from "udfs";
import { CopyTextButton } from "elements/CopyTextButton";
import { UdfType } from "system-udfs/convex/_system/frontend/common";

export function FunctionSummary({
  currentOpenFunction,
}: {
  currentOpenFunction: ModuleFunction;
}) {
  const [prodEditsEnabled, setProdEditsEnabled] = useSessionStorage(
    "prodEditsEnabled",
    false,
  );
  const [showEnableProdEditsModal, setShowEnableProdEditsModal] =
    useState(false);

  const npmPackageVersion = useQuery(udfs.getVersion.default);
  const versionTooOld = !!npmPackageVersion && lt(npmPackageVersion, "0.13.0");

  const deployment = useCurrentDeployment();
  const isProd = deployment?.deploymentType === "prod";
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canRunFunction =
    currentOpenFunction.udfType === "Query" || !isProd || hasAdminPermissions;

  const showGlobalRunner = useShowGlobalRunner();
  const showFunctionRunner = () => {
    showGlobalRunner(currentOpenFunction, "click");
  };

  const isPaused = useIsDeploymentPaused();
  if (isPaused === undefined) {
    return <Loading />;
  }
  return (
    <div className={classNames("flex h-full flex-col overflow-hidden")}>
      <div className="flex items-end justify-between gap-2 pb-2">
        {showEnableProdEditsModal && (
          <ProductionEditsConfirmationDialog
            onClose={() => {
              setShowEnableProdEditsModal(false);
            }}
            onConfirm={async () => {
              setProdEditsEnabled(true);
              setShowEnableProdEditsModal(false);
              showFunctionRunner();
            }}
          />
        )}
        <div className="flex items-center gap-2">
          <div className="flex flex-col items-start gap-1">
            <div className="flex items-center gap-3">
              <h3 className="font-mono">{currentOpenFunction.name}</h3>
              <div className="rounded border px-1 py-0.5 text-xs font-semibold text-content-primary">
                {functionTypeLabel(currentOpenFunction.udfType)}
              </div>
            </div>
            {currentOpenFunction.displayName !== currentOpenFunction.name && (
              <CopyTextButton
                className="font-mono"
                text={currentOpenFunction.displayName}
              />
            )}
          </div>
        </div>
        {
          // Supported UDF types for in-dashboard testing
          ["Query", "Mutation", "Action"].some(
            (udfType) => udfType === currentOpenFunction.udfType,
          ) && (
            <Button
              tip={
                !canRunFunction ? (
                  "You do not have permission to run this function in production."
                ) : isPaused ? (
                  <FunctionRunnerDisabledWhilePaused />
                ) : (
                  versionTooOld && (
                    <div>
                      The function runner is only available on deployments using
                      Convex version 0.13.0 or greater.
                    </div>
                  )
                )
              }
              disabled={isPaused || versionTooOld || !canRunFunction}
              onClick={() =>
                !isProd ||
                prodEditsEnabled ||
                currentOpenFunction.udfType === "Query"
                  ? showFunctionRunner()
                  : setShowEnableProdEditsModal(true)
              }
              icon={<PlayIcon />}
              size="sm"
              variant="primary"
            >
              Run Function
            </Button>
          )
        }
      </div>
    </div>
  );
}

export const functionTypeLabel = (udfType: UdfType) => {
  let text = "";
  switch (udfType) {
    case "Query":
      text = "Query";
      break;
    case "Mutation":
      text = "Mutation";
      break;
    case "Action":
      text = "Action";
      break;
    case "HttpAction":
      text = "HTTP";
      break;
    default:
      // eslint-disable-next-line no-case-declarations, @typescript-eslint/no-unused-vars
      const _typeCheck: never = udfType;
      text = "Function";
  }
  return text;
};
