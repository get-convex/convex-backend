import React, { useContext } from "react";
import { LocalDevCallout } from "@common/elements/LocalDevCallout";
import { Sheet } from "@ui/Sheet";
import {
  AuthIntegration,
  EXC_INTEGRATIONS,
  EXPORT_INTEGRATIONS,
  IMPORT_INTEGRATIONS,
  ExceptionReportingIntegration,
  LOG_INTEGRATIONS,
  LogIntegration,
} from "@common/lib/integrationHelpers";

import { Link } from "@ui/Link";
import {
  DeploymentInfo,
  DeploymentInfoContext,
  PermissionsContext,
} from "@common/lib/deploymentContext";
import { Doc } from "system-udfs/convex/_generated/dataModel";
import { PermissionDeniedTip } from "@common/elements/NoPermissionMessage";
import { PanelCard } from "./PanelCard";

export function Integrations({
  team,
  entitlements,
  integrations,
  workosData,
  onAddedIntegration,
}: {
  team: ReturnType<DeploymentInfo["useCurrentTeam"]>;
  entitlements: ReturnType<DeploymentInfo["useTeamEntitlements"]>;
  integrations: Doc<"_log_sinks">[];
  workosData: ReturnType<
    DeploymentInfo["workOSOperations"]["useDeploymentWorkOSEnvironment"]
  >["data"];
  onAddedIntegration?: (kind: string) => void;
}) {
  const { workosIntegrationEnabled } = useContext(DeploymentInfoContext);
  const { useIsOperationAllowed } = useContext(PermissionsContext);
  const canWriteIntegrations = useIsOperationAllowed("WriteIntegrations");

  const logStreamingEntitlementGranted = entitlements?.logStreamingEnabled;
  const streamingExportEntitlementGranted =
    entitlements?.streamingExportEnabled;

  const configuredIntegrationsMap = Object.fromEntries(
    integrations.map((integration) => [integration.config.type, integration]),
  );

  const logIntegrations: LogIntegration[] = LOG_INTEGRATIONS.map(
    (integrationKind) => {
      const existing = configuredIntegrationsMap[integrationKind];
      return {
        kind: integrationKind,
        existing: existing ?? null,
      } as LogIntegration;
    },
  );

  const authIntegrations: AuthIntegration[] = workosIntegrationEnabled
    ? [
        {
          kind: "workos",
          // Consider this integration to exist if a WorkOS environment has been provisioned
          existing: workosData?.environment ?? null,
        },
      ]
    : [];

  const exceptionReportingIntegrations: ExceptionReportingIntegration[] =
    EXC_INTEGRATIONS.map((kind) => {
      const existing = configuredIntegrationsMap[kind];
      return {
        kind,
        existing: existing ?? null,
      } as ExceptionReportingIntegration;
    });

  const devCallouts = [];
  if (!logStreamingEntitlementGranted) {
    devCallouts.push(
      <LocalDevCallout
        key="log-streaming"
        tipText="Tip: Run this to enable log streaming locally:"
        command={`just big-brain-tool-dev entitlement grant add --team-entitlement log_streaming_enabled --team-id ${team?.id} --reason "local" true --for-real`}
      />,
    );
  }
  if (!streamingExportEntitlementGranted) {
    devCallouts.push(
      <LocalDevCallout
        key="streaming-export"
        className="flex-col"
        tipText="Tip: Run this to enable streaming export locally:"
        command={`just big-brain-tool-dev entitlement grant add --team-entitlement streaming_export_enabled --team-id ${team?.id} --reason "local" true --for-real`}
      />,
    );
  }
  const logIntegrationUnvaliableReason = !logStreamingEntitlementGranted
    ? "MissingEntitlement"
    : !canWriteIntegrations
      ? "CannotManageDeployment"
      : null;

  const streamingExportIntegrationUnavailableReason =
    !streamingExportEntitlementGranted ? "MissingEntitlement" : null;

  // Precompute the tip so the permissionDeniedTip surface is consistent
  // across cards.
  const integrationWriteTip = (
    <PermissionDeniedTip
      message="You do not have permission to configure integrations on this deployment."
      action="deployment:integrations:write"
    />
  );

  // Show configured integrations first
  const allIntegrations = [
    ...authIntegrations,
    ...exceptionReportingIntegrations,
    ...logIntegrations,
  ].sort((a, b) => {
    if (a.existing !== null && b.existing === null) {
      return -1;
    }
    if (a.existing === null && b.existing !== null) {
      return 1;
    }
    return 0;
  });

  return (
    <div className="flex flex-col gap-4">
      <Sheet className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <h3>Integrations</h3>
          <div className="max-w-prose text-sm">
            Integrations allow you to send logs, report exceptions, and export
            Convex data to external services.{" "}
            <Link
              href="https://docs.convex.dev/production/integrations/"
              target="_blank"
            >
              Learn more
            </Link>{" "}
            about integrations.
          </div>
        </div>
        <div className="flex flex-col gap-2">
          {allIntegrations.map((i) => (
            <PanelCard
              key={i.kind}
              integration={i}
              unavailableReason={logIntegrationUnvaliableReason}
              teamSlug={team?.slug}
              onAddedIntegration={onAddedIntegration}
              writeDisabled={!canWriteIntegrations}
              writeDisabledTip={integrationWriteTip}
            />
          ))}
          {EXPORT_INTEGRATIONS.map((i) => (
            <PanelCard
              key={i}
              integration={{ kind: i }}
              unavailableReason={streamingExportIntegrationUnavailableReason}
              teamSlug={team?.slug}
              onAddedIntegration={onAddedIntegration}
              writeDisabled={!canWriteIntegrations}
              writeDisabledTip={integrationWriteTip}
            />
          ))}
          {IMPORT_INTEGRATIONS.map((i) => (
            <PanelCard
              key={i}
              integration={{ kind: i }}
              unavailableReason={null}
              teamSlug={team?.slug}
              onAddedIntegration={onAddedIntegration}
            />
          ))}
        </div>
      </Sheet>
      {devCallouts}
    </div>
  );
}
