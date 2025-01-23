import classNames from "classnames";
import { useQuery } from "convex/react";
import { useRouter } from "next/router";
import { useEffect, useCallback } from "react";
import { usePrevious } from "react-use";
import udfs from "udfs";
import { useHotkeys } from "react-hotkeys-hook";
import { Button } from "../../../elements/Button";
import { FunctionIcon } from "../../../elements/icons";
import { useLogDeploymentEvent } from "../../../lib/deploymentApi";
import { toast } from "../../../lib/utils";
import {
  useHideGlobalRunner,
  useIsGlobalRunnerShown,
  useShowGlobalRunner,
  useGlobalRunnerSelectedItem,
} from "../lib/functionRunner";
import { GlobalFunctionTester } from "./FunctionTester";

export function FunctionRunnerWrapper({
  isVertical,
  setIsVertical,
  isExpanded,
  setIsExpanded,
}: {
  isVertical: boolean;
  setIsVertical: (isVertical: boolean) => void;
  isExpanded: boolean;
  setIsExpanded: (isExpanded: boolean) => void;
}) {
  const deploymentState = useQuery(udfs.deploymentState.deploymentState);
  const router = useRouter();
  const { query } = router;
  const previousQuery = usePrevious(query);

  const isGlobalRunnerShown = useIsGlobalRunnerShown();
  const showGlobalRunner = useShowGlobalRunner();
  const hideGlobalRunner = useHideGlobalRunner();
  const [globalRunnerSelectedItem, setGlobalRunnerSelectedItem] =
    useGlobalRunnerSelectedItem();

  useEffect(() => {
    if (
      previousQuery?.team !== query.team ||
      previousQuery?.project !== query.project ||
      previousQuery?.deploymentName !== query.deploymentName
    ) {
      if (isGlobalRunnerShown) {
        hideGlobalRunner("redirect");
      }
      if (globalRunnerSelectedItem !== null) {
        setGlobalRunnerSelectedItem(null);
      }
    }
  }, [
    previousQuery?.team,
    previousQuery?.project,
    previousQuery?.deploymentName,
    query.team,
    query.project,
    query.deploymentName,
    query.deployment,
    hideGlobalRunner,
    isGlobalRunnerShown,
    globalRunnerSelectedItem,
    setGlobalRunnerSelectedItem,
  ]);

  useHotkeys("ctrl+`", () => {
    if (deploymentState?.state === "paused") {
      toast(
        "error",
        "You cannot run functions while the deployment is paused.",
      );
      return;
    }
    isGlobalRunnerShown
      ? hideGlobalRunner("keyboard")
      : showGlobalRunner(null, "keyboard");
  });

  const log = useLogDeploymentEvent();

  const setIsGlobalRunnerVerticalAndLog = useCallback(
    (vertical: boolean) => {
      setIsVertical(vertical);
      log("change function runner orientation", {
        orientation: vertical ? "vertical" : "horizontal",
      });
    },
    [log, setIsVertical],
  );

  return (
    <>
      <GlobalFunctionTester
        isVertical={!!isVertical}
        setIsVertical={setIsGlobalRunnerVerticalAndLog}
        isExpanded={isExpanded}
        setIsExpanded={setIsExpanded}
      />
      {!isGlobalRunnerShown && deploymentState?.state !== "paused" && (
        <Button
          variant="unstyled"
          className={classNames(
            "fixed bottom-16 right-0",
            "group flex items-center gap-2 rounded-l-2xl border-border-selected border-y border-l bg-background-secondary p-2",
            !isGlobalRunnerShown && "w-12 hover:w-40",
          )}
          onClick={() => {
            showGlobalRunner(null, "click");
          }}
        >
          <div className="h-8 w-8">
            <FunctionIcon className="h-8 w-8" />
          </div>

          <div className="ml-1 select-none flex-col items-center gap-1 whitespace-nowrap  transition-all">
            Run functions
            <div className="flex w-full items-center gap-0.5 text-xs">
              Shortcut:
              <kbd className="font-sans font-semibold">⌃</kbd> +
              <kbd className="font-sans font-semibold">`</kbd>
            </div>
          </div>
        </Button>
      )}
    </>
  );
}
