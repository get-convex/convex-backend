import {
  ChevronLeftIcon,
  ChevronRightIcon,
  CodeIcon,
} from "@radix-ui/react-icons";
import {
  Button,
  DeploymentInfoContext,
  useGlobalLocalStorage,
} from "dashboard-common";
import { UserIdentityAttributes } from "convex/browser";
import { Value } from "convex/values";
import isEqual from "lodash/isEqual";
import cloneDeep from "lodash/cloneDeep";
import omit from "lodash/omit";
import { useContext, useEffect, useState } from "react";
import { createGlobalState } from "react-use";
import {
  useFunctionUrl,
  useLogDeploymentEvent,
} from "../../../lib/deploymentApi";
import { ComponentId } from "../../../lib/useNents";

// Keep track of a single user across instances of FunctionTester
export const useImpersonatedUser = createGlobalState<UserIdentityAttributes>({
  subject: "fake_id",
  issuer: "fake_issuer",
});

export const useIsImpersonating = createGlobalState<boolean | undefined>();

export function RunHistory({
  functionIdentifier,
  componentId,
  selectItem,
}: {
  functionIdentifier: string;
  componentId: ComponentId;
  selectItem: (item: RunHistoryItem) => void;
}) {
  const log = useLogDeploymentEvent();
  const url = useFunctionUrl(functionIdentifier, componentId);
  const { runHistory } = useRunHistory(functionIdentifier, componentId);
  const [currentIdx, setCurrentIdx] = useState(0);

  useEffect(() => {
    setCurrentIdx(0);
  }, [runHistory]);
  return (
    <div className="flex gap-2">
      <Button
        icon={<CodeIcon />}
        size="xs"
        variant="neutral"
        tip="Jump to Code"
        href={`${url}#code`}
        onClick={() => log("jump to code from function runner")}
      />
      <Button
        onClick={() => {
          selectItem(runHistory[currentIdx + 1]);
          setCurrentIdx(currentIdx + 1);
        }}
        disabled={currentIdx + 1 >= runHistory.length}
        icon={<ChevronLeftIcon />}
        size="xs"
        variant="neutral"
        tip="Previous Arguments"
      />
      <Button
        onClick={() => {
          selectItem(runHistory[currentIdx - 1]);
          setCurrentIdx(currentIdx - 1);
        }}
        disabled={currentIdx <= 0}
        icon={<ChevronRightIcon />}
        size="xs"
        variant="neutral"
        tip="Next Arguments"
      />
    </div>
  );
}

export type RunHistoryItem = {
  startedAt: number;
  endedAt: number;
} & (
  | {
      type: "arguments";
      arguments: Record<string, Value>;
      user?: UserIdentityAttributes;
    }
  | { type: "custom"; code: string }
);

export function useRunHistory(
  fnName: string,
  componentId: ComponentId,
): {
  runHistory: RunHistoryItem[];
  appendRunHistory: (value: RunHistoryItem) => void;
} {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const [runHistory, setRunHistory] = useGlobalLocalStorage(
    `runHistory/${deployment?.name}/${componentId ? `${componentId}/` : ""}${fnName}`,
    [] as RunHistoryItem[],
  );

  const [isImpersonating] = useIsImpersonating();
  const [impersonatedUser] = useImpersonatedUser();
  return {
    runHistory,
    appendRunHistory: (value) => {
      if (
        runHistory.length > 0 &&
        isEqual(
          omit(runHistory[0], ["endedAt", "startedAt", "user"]),
          omit(value, ["endedAt", "startedAt", "user"]),
        ) &&
        runHistory[0].type === "arguments" &&
        value.type === "arguments" &&
        isEqual(runHistory[0].user, impersonatedUser)
      ) {
        return;
      }
      setRunHistory((prev: RunHistoryItem[]) => {
        const newValue = cloneDeep(value);

        if (newValue.type === "arguments" && isImpersonating) {
          newValue.user = impersonatedUser;
        }

        const updatedHistory = [newValue, ...prev];
        if (updatedHistory.length > 25) {
          updatedHistory.pop();
        }
        return updatedHistory;
      });
    },
  };
}
