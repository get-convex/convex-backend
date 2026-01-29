import { PlayIcon } from "@radix-ui/react-icons";
import { useQuery } from "convex/react";
import { useContext, useState } from "react";
import { lt } from "semver";
import udfs from "@common/udfs";
import { UdfType } from "system-udfs/convex/_system/frontend/common";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { FunctionRunnerDisabledWhilePaused } from "@common/features/functions/components/FunctionRunnerDisabledWhilePaused";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useShowGlobalRunner } from "@common/features/functionRunner/lib/functionRunner";
import { ModuleFunction } from "@common/lib/functions/types";
import { Loading } from "@ui/Loading";
import { AuthorizeEditsConfirmationDialog } from "@common/elements/AuthorizeEditsConfirmationDialog";
import { Button } from "@ui/Button";
import { useEditsAuthorization } from "@common/features/data/lib/useEditsAuthorization";

export function FunctionSummary({
  currentOpenFunction,
}: {
  currentOpenFunction: ModuleFunction;
}) {
  const { areEditsAuthorized, authorizeEdits } = useEditsAuthorization();
  const [showAuthorizeEditsModal, setShowAuthorizeEditsModal] = useState(false);

  const npmPackageVersion = useQuery(udfs.getVersion.default);
  const versionTooOld = !!npmPackageVersion && lt(npmPackageVersion, "0.13.0");

  const {
    useCurrentDeployment,
    useHasProjectAdminPermissions,
    useIsDeploymentPaused,
  } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const isProd = deployment?.deploymentType === "prod";
  const canRunFunction =
    // TODO(ENG-10284) Make this depend on permissions, not deployment type
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
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex flex-wrap items-end justify-between gap-2 pb-2">
        {showAuthorizeEditsModal && (
          <AuthorizeEditsConfirmationDialog
            onClose={() => {
              setShowAuthorizeEditsModal(false);
            }}
            onConfirm={async () => {
              authorizeEdits?.();
              setShowAuthorizeEditsModal(false);
              showFunctionRunner();
            }}
          />
        )}
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex flex-wrap items-center gap-x-2">
            <h3 className="font-mono">{currentOpenFunction.name}</h3>
            <div
              className={`rounded-sm p-1 text-xs font-semibold ${getFunctionTypeStyles(currentOpenFunction.udfType).text} ${getFunctionTypeStyles(currentOpenFunction.udfType).background}`}
            >
              {currentOpenFunction.visibility.kind === "internal" &&
                "Internal "}
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
                currentOpenFunction.udfType === "Query" || areEditsAuthorized
                  ? showFunctionRunner()
                  : setShowAuthorizeEditsModal(true)
              }
              icon={<PlayIcon />}
              size="xs"
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
      udfType satisfies never;
      text = "Function";
  }
  return text;
};

export const getFunctionTypeStyles = (udfType: UdfType) => {
  switch (udfType) {
    case "Query":
      return {
        text: "text-yellow-700 dark:text-yellow-200",
        background: "bg-yellow-100/50 dark:bg-yellow-900/50",
      };
    case "Mutation":
      return {
        text: "text-blue-700 dark:text-blue-200",
        background: "bg-blue-100/50 dark:bg-blue-900/50",
      };
    case "Action":
      return {
        text: "text-purple-700 dark:text-purple-200",
        background: "bg-purple-100/50 dark:bg-purple-900/50",
      };
    case "HttpAction":
      return {
        text: "text-green-700 dark:text-green-200",
        background: "bg-green-100/50 dark:bg-green-900/50",
      };
    default:
      return {
        text: "text-content-primary",
        background: "bg-background-tertiary",
      };
  }
};
