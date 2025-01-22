import { useCallback } from "react";
import { createGlobalState, useLocalStorage } from "react-use";
import { useRouter } from "next/router";
import {
  useModuleFunctions,
  displayNameToIdentifier,
} from "../../../lib/functions/FunctionsProvider";
import { ModuleFunction } from "../../../lib/functions/types";
import { useLogDeploymentEvent } from "../../../lib/deploymentApi";
import { ComponentId, useNents } from "../../../lib/useNents";
import { useTableMetadata } from "../../../lib/useTableMetadata";

export const useCurrentGloballyOpenFunction =
  createGlobalState<ModuleFunction | null>(null);

export type CustomQuery = {
  type: "customQuery";
  table: string | null;
};

const useGlobalRunnerShown = createGlobalState(false);
export const useGlobalRunnerSelectedItem = createGlobalState<{
  componentId: ComponentId;
  fn: ModuleFunction | CustomQuery;
} | null>(null);

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
  const log = useLogDeploymentEvent();
  const [isGlobalRunnerVertical] = useLocalStorage(
    "functionRunnerOrientation",
    false,
  );

  return useCallback(
    (
      selected: ModuleFunction | CustomQuery | null,
      how: "click" | "keyboard" | "tutorial" | "redirect",
    ) => {
      log(`open function runner`, {
        how,
        orientation: isGlobalRunnerVertical ? "vertical" : "horizontal",
        function: selected?.type !== "customQuery" &&
          selected !== null && {
            udfType: selected.udfType,
            visibility: selected.visibility,
            identifier: selected.identifier,
          },
        customQuery: selected?.type === "customQuery",
      });
      if (selected || !selectedItem) {
        const fn = selected ??
          currentlyOpenFunction ??
          defaultFunction ?? {
            type: "customQuery",
            table: tableMetadata?.name ?? null,
          };
        setGlobalRunnerSelectedItem({
          componentId:
            fn.type === "customQuery"
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
