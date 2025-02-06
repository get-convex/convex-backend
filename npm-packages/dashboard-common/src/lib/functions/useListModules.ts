import { useQuery } from "convex/react";
import { useMemo } from "react";
import udfs from "@common/udfs";
import { Module } from "system-udfs/convex/_system/frontend/common";
import { useInvalidateSourceCode } from "@common/lib/deploymentApi";
import { ComponentId, useNents } from "@common/lib/useNents";

export function useListModules(): Map<string, Module> | undefined {
  const { selectedNent } = useNents();
  const modulesAllNents = useListModulesAllNents();

  return useMemo(
    () => modulesAllNents?.get(selectedNent?.id ?? null) ?? undefined,
    [modulesAllNents, selectedNent],
  );
}

export function useListModulesAllNents():
  | Map<ComponentId | null, Map<string, Module>>
  | undefined {
  const rawModules = useQuery(udfs.modules.listForAllComponents);

  const invalidateSourceCode = useInvalidateSourceCode();
  const allModules: Map<ComponentId | null, Map<string, Module>> | undefined =
    useMemo(() => {
      if (rawModules === undefined) {
        return undefined;
      }

      const allModulesMap = new Map<ComponentId | null, Map<string, Module>>();
      for (const [componentId, modules] of rawModules) {
        allModulesMap.set(
          componentId as ComponentId | null,
          new Map(modules as [string, Module][]),
        );

        for (const [path, _] of modules) {
          void invalidateSourceCode(componentId, path);
        }
      }

      return allModulesMap;
    }, [invalidateSourceCode, rawModules]);

  return allModules;
}
