import { Tooltip } from "@ui/Tooltip";
import { ReactNode } from "react";
import { IntegrationType } from "system-udfs/convex/_system/frontend/common";

export function IntegrationTitle({
  logo,
  integrationKind,
  description,
}: {
  logo: ReactNode;
  integrationKind: IntegrationType;
  description: ReactNode;
}) {
  return (
    <div className="flex items-center gap-2">
      {logo}

      <p className="text-sm font-semibold">
        {integrationKind === "workos"
          ? "WorkOS"
          : integrationKind.charAt(0).toUpperCase() + integrationKind.slice(1)}
      </p>
      <Tooltip tip={description}>
        <p className="max-w-fit rounded-sm border p-1 text-xs">
          {integrationKind === "sentry"
            ? "Exception Reporting"
            : integrationKind === "airbyte" || integrationKind === "fivetran"
              ? "Streaming Export"
              : integrationKind === "workos"
                ? "Authentication"
                : "Log Stream"}
        </p>
      </Tooltip>
    </div>
  );
}
