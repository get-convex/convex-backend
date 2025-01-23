import { captureMessage } from "@sentry/nextjs";
import { useEffect } from "react";
import { usePrevious } from "react-use";
import { toast as sonnerToast } from "sonner";

export function dismissToast(id: string) {
  sonnerToast.dismiss(id);
}

// utility for logging changed values in useEffect re-renders
export const useEffectDebugger = (
  effectHook: Parameters<typeof useEffect>[0],
  dependencies: Parameters<typeof useEffect>[1],
  dependencyNames = [],
) => {
  const previousDeps = usePrevious(dependencies);

  const changedDeps =
    dependencies?.reduce(
      (acc: Record<string, { before: any; after: any }>, dependency, index) => {
        if (previousDeps && dependency !== previousDeps[index]) {
          const keyName = dependencyNames[index] || index;
          return {
            ...acc,
            [keyName]: {
              before: previousDeps[index],
              after: dependency,
            },
          };
        }

        return acc;
      },
      {},
    ) || {};

  if (Object.keys(changedDeps).length) {
    // eslint-disable-next-line no-console
    console.log("[useEffectDebugger] ", changedDeps);
  }

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(effectHook, dependencies);
};

export const reportHttpError = (
  method: string,
  url: string,
  error: { code: string; message: string },
) => {
  captureMessage(
    `failed to request ${method} ${url}: ${error.code} - ${error.message} `,
  );
};
