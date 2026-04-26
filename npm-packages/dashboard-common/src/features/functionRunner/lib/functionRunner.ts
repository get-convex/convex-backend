import { useCallback, useContext } from "react";
import { createGlobalState, useLocalStorage } from "react-use";
import { useRouter } from "next/router";
import {
  useModuleFunctions,
  displayNameToIdentifier,
} from "@common/lib/functions/FunctionsProvider";
import { ModuleFunction } from "@common/lib/functions/types";
import { ComponentId, useNents } from "@common/lib/useNents";
import { useTableMetadata } from "@common/lib/useTableMetadata";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export const useCurrentGloballyOpenFunction =
  createGlobalState<ModuleFunction | null>(null);

export type CustomFunction = {
  type: "customQuery" | "customMutation";
  table: string | null;
};

const useGlobalRunnerShown = createGlobalState(false);
export const useGlobalRunnerSelectedItem = createGlobalState<{
  componentId: ComponentId;
  fn: ModuleFunction | CustomFunction;
} | null>(null);

export function isCustomFunction(
  fn: ModuleFunction | CustomFunction,
): fn is CustomFunction {
  return "type" in fn;
}

export function useIsGlobalRunnerShown() {
  const [isShown] = useGlobalRunnerShown();
  return isShown;
}

export function useShowGlobalRunner() {
  const [, setGlobalRunnerShown] = useGlobalRunnerShown();
  const [selectedItem, setGlobalRunnerSelectedItem] =
    useGlobalRunnerSelectedItem();
  const { selectedNent } = useNents();
  const defaultFunction = useDefaultFunction();
  const [currentlyOpenFunction] = useCurrentGloballyOpenFunction();
  const tableMetadata = useTableMetadata();

  // only for logging
  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();
  const [isGlobalRunnerVertical] = useLocalStorage(
    "functionRunnerOrientation",
    false,
  );

  return useCallback(
    (
      selected: ModuleFunction | CustomFunction | null,
      how: "click" | "keyboard" | "tutorial" | "redirect",
    ) => {
      log(`open function runner`, {
        how,
        orientation: isGlobalRunnerVertical ? "vertical" : "horizontal",
        function: selected !== null &&
          !isCustomFunction(selected) && {
            udfType: selected.udfType,
            visibility: selected.visibility,
            identifier: selected.identifier,
          },
        customFunction:
          selected !== null && isCustomFunction(selected)
            ? selected.type
            : undefined,
      });
      if (selected || !selectedItem) {
        const fn = selected ??
          currentlyOpenFunction ??
          defaultFunction ?? {
            type: "customQuery",
            table: tableMetadata?.name ?? null,
          };
        setGlobalRunnerSelectedItem({
          componentId: isCustomFunction(fn)
            ? (selectedNent?.id ?? null)
            : fn.componentId,
          fn,
        });
      }
      setGlobalRunnerShown(true);
    },
    [
      log,
      isGlobalRunnerVertical,
      selectedItem,
      setGlobalRunnerShown,
      currentlyOpenFunction,
      defaultFunction,
      tableMetadata?.name,
      setGlobalRunnerSelectedItem,
      selectedNent,
    ],
  );
}

export function useHideGlobalRunner() {
  const [, setGlobalRunnerShown] = useGlobalRunnerShown();
  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();
  return useCallback(
    (how: "click" | "redirect" | "keyboard") => {
      log(`close function runner`, { how });
      setGlobalRunnerShown(false);
    },
    [log, setGlobalRunnerShown],
  );
}

function useDefaultFunction() {
  const { query } = useRouter();
  const moduleFunctions = useModuleFunctions();
  const { selectedNent } = useNents();
  const defaultFunction = query.function?.toString();
  return (
    (defaultFunction !== undefined
      ? findFunction(
          moduleFunctions,
          displayNameToIdentifier(defaultFunction),
          selectedNent?.id ?? null,
        )
      : findFirstWritingFunction(moduleFunctions, selectedNent?.id ?? null)) ??
    null
  );
}

export function findFirstWritingFunction(
  functions: ModuleFunction[],
  selectedNentId: ComponentId,
) {
  return functions.find(
    (item) => isWritingFunction(item) && item.componentId === selectedNentId,
  );
}

function isWritingFunction(fn: ModuleFunction) {
  return fn.udfType === "Mutation" || fn.udfType === "Action";
}

export function findFunction(
  functions: ModuleFunction[],
  identifier: string,
  componentId: ComponentId,
) {
  return functions.find(
    (value) =>
      value.identifier === identifier && value.componentId === componentId,
  );
}
