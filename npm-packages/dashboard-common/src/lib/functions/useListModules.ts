import { useQuery } from "convex/react";
import { useContext, useMemo } from "react";
import udfs from "@common/udfs";
import { Module } from "system-udfs/convex/_system/frontend/common";
import { ComponentId, useNents } from "@common/lib/useNents";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

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
  const {
    useCurrentDeployment,
    useHasProjectAdminPermissions,
    useIsOperationAllowed,
    useCustomRolePermission,
  } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canViewData = useIsOperationAllowed("ViewData");
  // Built-in admin/developer members keep the admin-key gate; custom-role
  // members need an explicit `deployment:data:view` grant. A project
  // admin always passes.
  const canViewDataCustom = useCustomRolePermission(
    "deployment:data:view",
    true,
  );
  const canView =
    canViewData && (hasAdminPermissions || canViewDataCustom !== false);
  const rawModulesOrSkipped = useQuery(
    udfs.modules.listForAllComponents,
    canView ? {} : "skip",
  );
  const rawModules = useMemo(
    () => (canView ? rawModulesOrSkipped : []),
    [canView, rawModulesOrSkipped],
  );

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
      }

      return allModulesMap;
    }, [rawModules]);

  return allModules;
}
