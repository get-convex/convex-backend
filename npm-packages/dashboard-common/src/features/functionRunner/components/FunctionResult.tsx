import { LockOpen2Icon, PlayIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { Button, DeploymentInfoContext, toast } from "dashboard-common";
import type { FunctionResult as FunctionResultType } from "convex/browser";
import { useContext, useEffect, useState } from "react";
import { useSessionStorage } from "react-use";
import { Value } from "convex/values";
import { useLogDeploymentEvent } from "../../../lib/deploymentApi";
import { RequestFilter } from "../../../lib/appMetrics";
import { Spinner } from "../../../elements/Spinner";
import { ComponentId } from "../../../lib/useNents";
import { Result } from "./Result";
import {
  useRunHistory,
  RunHistoryItem,
  useImpersonatedUser,
  useIsImpersonating,
} from "./RunHistory";

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
        runHistoryItem.user && setImpersonatedUser(runHistoryItem.user);
      }
    }
  }, [runHistoryItem, setImpersonatedUser, setIsImpersonating]);

  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );

  const deployment = useCurrentDeployment();
  const isProd = deployment?.deploymentType === "prod";
  const [prodEditsEnabled, setProdEditsEnabled] = useSessionStorage(
    "prodEditsEnabled",
    false,
  );
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
              : isProd && !prodEditsEnabled
                ? `You are about to run a ${udfType.toLowerCase()} in Production. Unlock Production to continue.`
                : !canRunFunction
                  ? "You do not have permission to run this function in production."
                  : undefined
          }
          size="sm"
          className="w-full max-w-[48rem] items-center justify-center"
          disabled={
            isInFlight ||
            disabled ||
            (isProd && !prodEditsEnabled) ||
            !canRunFunction
          }
          onClick={runFunction}
          icon={isInFlight ? <Spinner /> : <PlayIcon />}
        >
          Run {udfType.toLowerCase()}
        </Button>
        {canRunFunction && isProd && !prodEditsEnabled && (
          <Button
            tip="Enables changes to Production for the remainder of this dashboard session"
            size="sm"
            onClick={() => {
              setProdEditsEnabled(true);
              toast(
                "success",
                "Production edits enabled for the remainder of this dashboard session",
              );
            }}
            icon={<LockOpen2Icon />}
          >
            Unlock Production
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
      />
    ),
    runFunction,
  };
}
