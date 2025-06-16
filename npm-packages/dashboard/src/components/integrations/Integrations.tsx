import React from "react";
import { Integration } from "system-udfs/convex/_system/frontend/common";
import { Team, TeamEntitlementsResponse } from "generatedApi";
import { LocalDevCallout } from "@common/elements/LocalDevCallout";
import { Callout } from "@ui/Callout";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import {
  EXC_INTEGRATIONS,
  EXPORT_INTEGRATIONS,
  ExceptionReportingIntegration,
  LOG_INTEGRATIONS,
  LogIntegration,
} from "@common/lib/integrationHelpers";

import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import Link from "next/link";
import { PanelCard } from "./PanelCard";

export function Integrations({
  team,
  entitlements,
  integrations,
}: {
  team: Team;
  entitlements: TeamEntitlementsResponse;
  integrations: Integration[];
}) {
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const cannotManageBecauseProd =
    deployment?.deploymentType === "prod" && !hasAdminPermissions;
  const isLocalDeployment = deployment?.kind === "local";

  const logStreamingEntitlementGranted = entitlements?.logStreamingEnabled;
  const streamingExportEntitlementGranted =
    entitlements?.streamingExportEnabled;

  // Sort the configured and unconfigured integrations in the order specified by LOG_INTEGRATIONS
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
  ).sort((a: LogIntegration, b: LogIntegration) => {
    // Show configured integrations first
    if (a.existing !== null && b.existing === null) {
      return -1;
    }
    return 0;
  });

  const exceptionReportingIntegrations: ExceptionReportingIntegration[] =
    EXC_INTEGRATIONS.map((kind) => {
      const existing = configuredIntegrationsMap[kind];
      return {
        kind,
        existing: existing ?? null,
      } as ExceptionReportingIntegration;
    }).sort((a, b) => {
      // Show configured integrations first
      if (a.existing !== null && b.existing === null) {
        return -1;
      }
      return 0;
    });

  // Show the proCallout if either of the entitlements aren't granted. Both are granted
  // with a pro account.
  const proCallout =
    logStreamingEntitlementGranted &&
    streamingExportEntitlementGranted ? null : (
      <Callout variant="upsell">
        <div className="flex w-fit flex-col gap-2">
          <p className="max-w-prose">
            Log Stream, Exception Reporting, and Streaming Export integrations
            are available on the Pro plan.
          </p>
          <Button
            href={`/${team.slug}/settings/billing`}
            size="xs"
            className="w-fit"
          >
            Upgrade Now
          </Button>
        </div>
      </Callout>
    );

  const devCallouts = [];
  if (!logStreamingEntitlementGranted) {
    devCallouts.push(
      <LocalDevCallout
        tipText="Tip: Run this to enable log streaming locally:"
        command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement log_streaming_enabled --team-id ${team.id} --reason "local" true --for-real`}
      />,
    );
  }
  if (!streamingExportEntitlementGranted) {
    devCallouts.push(
      <LocalDevCallout
        className="flex-col"
        tipText="Tip: Run this to enable streaming export locally:"
        command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement streaming_export_enabled --team-id ${team.id} --reason "local" true --for-real`}
      />,
    );
  }
  const logIntegrationUnvaliableReason = !logStreamingEntitlementGranted
    ? "MissingEntitlement"
    : cannotManageBecauseProd
      ? "CannotManageProd"
      : isLocalDeployment
        ? "LocalDeployment"
        : null;

  const streamingExportIntegrationUnavailableReason =
    !streamingExportEntitlementGranted
      ? "MissingEntitlement"
      : isLocalDeployment
        ? "LocalDeployment"
        : null;

  return (
    <div className="flex flex-col gap-4">
      {proCallout}
      <Sheet className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <h3>Integrations</h3>
          <div className="max-w-prose text-sm">
            Integrations allow you to send logs, report exceptions, and export
            Convex data to external services.{" "}
            <Link
              href="https://docs.convex.dev/production/integrations/"
              target="_blank"
              className="text-content-link hover:underline"
            >
              Learn more
            </Link>{" "}
            about integrations.
          </div>
        </div>
        <div className="flex flex-col gap-2">
          {[...exceptionReportingIntegrations, ...logIntegrations]
            .sort((a, b) =>
              a.existing !== null && b.existing === null ? -1 : 0,
            )
            .map((i) => (
              <PanelCard
                integration={i}
                unavailableReason={logIntegrationUnvaliableReason}
              />
            ))}
          {EXPORT_INTEGRATIONS.map((i) => (
            <PanelCard
              integration={{ kind: i }}
              unavailableReason={streamingExportIntegrationUnavailableReason}
            />
          ))}
        </div>
      </Sheet>
      {devCallouts}
    </div>
  );
}
