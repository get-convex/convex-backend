import { LockOpen2Icon, PlayIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import type { FunctionResult as FunctionResultType } from "convex/browser";
import { useContext, useEffect, useState } from "react";
import { Value } from "convex/values";
import { Button } from "@ui/Button";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { toast } from "@common/lib/utils";
import { RequestFilter } from "@common/lib/appMetrics";
import { ComponentId } from "@common/lib/useNents";
import { Result } from "@common/features/functionRunner/components/Result";
import {
  useRunHistory,
  RunHistoryItem,
  useImpersonatedUser,
  useIsImpersonating,
} from "@common/features/functionRunner/components/RunHistory";
import { useEditsAuthorization } from "@common/features/data/lib/useEditsAuthorization";

// This is a hook because we want to return composable components that can be arranged
// vertically or horizontally.
export function useFunctionResult({
  udfType,
  onSubmit,
  disabled,
  functionIdentifier,
  componentId,
  args,
  runHistoryItem,
  onCopiedQueryResult,
}: {
  udfType?: "Mutation" | "Action" | "Query" | "HttpAction";
  onSubmit(): {
    requestFilter: RequestFilter | null;
    runFunctionPromise: Promise<FunctionResultType> | null;
  };
  disabled: boolean;
  functionIdentifier?: string;
  componentId: ComponentId;
  args: Record<string, Value>;
  runHistoryItem?: RunHistoryItem;
  onCopiedQueryResult?: () => void;
}) {
  const { appendRunHistory } = useRunHistory(
    functionIdentifier || "",
    componentId,
  );
  const [isInFlight, setIsInFlight] = useState(false);
  const [lastRequestTiming, setLastRequestTiming] = useState<{
    startedAt: number;
    endedAt: number;
  }>();

  const [, setIsImpersonating] = useIsImpersonating();
  const [, setImpersonatedUser] = useImpersonatedUser();

  const [result, setResult] = useState<FunctionResultType>();
  const [requestFilter, setRequestFilter] = useState<RequestFilter | null>(
    null,
  );
  const [startCursor, setStartCursor] = useState<number>(0);

  const isInvalidUdfType =
    !udfType || !["Mutation", "Action"].includes(udfType);
  useEffect(() => {
    if (!isInvalidUdfType) {
      setResult(undefined);
      setLastRequestTiming(undefined);
      setIsInFlight(false);
    }
  }, [isInvalidUdfType]);

  useEffect(() => {
    setResult(undefined);
    setLastRequestTiming(undefined);
    setIsInFlight(false);
  }, [functionIdentifier]);

  useEffect(() => {
    if (runHistoryItem) {
      setResult(undefined);
      setLastRequestTiming(undefined);
      setStartCursor(0);
      if (runHistoryItem.type === "arguments") {
        setIsImpersonating(!!runHistoryItem.user);
        if (runHistoryItem.user) {
          setImpersonatedUser(runHistoryItem.user);
        }
      }
    }
  }, [runHistoryItem, setImpersonatedUser, setIsImpersonating]);

  const {
    useCurrentDeployment,
    useHasProjectAdminPermissions,
    useLogDeploymentEvent,
  } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();
  const dtype = deployment?.deploymentType;
  const isProd = dtype === "prod";

  const { areEditsAuthorized, authorizeEdits } = useEditsAuthorization();
  const log = useLogDeploymentEvent();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canRunFunction =
    udfType === "Query" ||
    deployment?.deploymentType !== "prod" ||
    hasAdminPermissions;

  if (isInvalidUdfType) {
    return { button: null, result: null };
  }
  const runFunction = async () => {
    const startedAt = Date.now();
    setResult(undefined);
    setIsInFlight(true);
    const { requestFilter: requestFilterResult, runFunctionPromise } =
      onSubmit();
    setRequestFilter(requestFilterResult);
    setStartCursor(startedAt);
    let functionResult: FunctionResultType | undefined;
    try {
      functionResult = await runFunctionPromise!;
      log("run function", {
        function: {
          identifier: functionIdentifier,
          udfType,
        },
        success: functionResult.success,
        isProd,
      });
    } catch (e: any) {
      functionResult = {
        success: false,
        errorMessage: e.message,
        logLines: [],
      };
    } finally {
      // Wait a moment before re-enable the button to
      // avoid the user accidently re-running the function.
      setTimeout(() => {
        setIsInFlight(false);
      }, 100);
      const endedAt = Date.now();
      setLastRequestTiming({
        startedAt,
        endedAt,
      });
      setResult(functionResult);
      appendRunHistory({
        type: "arguments",
        startedAt,
        endedAt,
        arguments: args,
      });
    }
  };

  return {
    button: (
      <div className={classNames("flex items-center gap-2 mx-4")}>
        <Button
          tip={
            disabled
              ? "Fix the errors above to continue."
              : !canRunFunction
                ? "You do not have permission to run this function in production."
                : !areEditsAuthorized
                  ? // TODO(ENG-10340) Edit this message to use the deployment ref
                    `You are about to run a ${udfType.toLowerCase()} in a ${`${dtype ?? ""} deployment`.trim()}. Unlock edits to continue.`
                  : undefined
          }
          size="sm"
          className="w-full max-w-[48rem] items-center justify-center"
          disabled={disabled || !areEditsAuthorized || !canRunFunction}
          loading={isInFlight}
          onClick={runFunction}
          icon={<PlayIcon />}
        >
          Run {udfType.toLowerCase()}
        </Button>
        {canRunFunction && !areEditsAuthorized && (
          // TODO(ENG-10340) Include the deployment ref in the tooltip
          <Button
            tip="Enables changes to this deployment for the remainder of this dashboard session"
            size="sm"
            onClick={() => {
              authorizeEdits?.();
              toast(
                "success",
                "Edits enabled for the remainder of this dashboard session",
              );
            }}
            icon={<LockOpen2Icon />}
          >
            Unlock Edits
          </Button>
        )}
      </div>
    ),
    result: (
      <Result
        result={result}
        loading={isInFlight}
        lastRequestTiming={lastRequestTiming}
        requestFilter={requestFilter}
        startCursor={startCursor}
        onCopiedQueryResult={onCopiedQueryResult}
      />
    ),
    runFunction,
  };
}
