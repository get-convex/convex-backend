import { useQuery } from "convex/react";
import Link from "next/link";
import udfs from "@common/udfs";
import { useRouter } from "next/router";
import {
  IntegrationConfig,
  IntegrationType,
} from "system-udfs/convex/_system/frontend/common";
import { ExternalLinkIcon, GearIcon } from "@radix-ui/react-icons";
import { useContext } from "react";
import { HealthCard } from "@common/elements/HealthCard";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Loading } from "@ui/Loading";
import {
  integrationName,
  configToUrl,
  integrationUsingLegacyFormat,
  integrationToLogo,
} from "@common/lib/integrationHelpers";

export function IntegrationStatus({
  integrationTypes,
  title,
  notConfiguredSummary,
}: {
  integrationTypes: IntegrationConfig["type"][];
  title: string;
  notConfiguredSummary: React.ReactNode;
}) {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();

  if (deployment?.kind === "local") {
    return null;
  }
  return (
    <IntegrationStatusCard
      integrationTypes={integrationTypes}
      title={title}
      notConfiguredSummary={notConfiguredSummary}
    />
  );
}

function IntegrationStatusCard({
  integrationTypes,
  title,
  notConfiguredSummary,
}: {
  integrationTypes: IntegrationConfig["type"][];
  title: string;
  notConfiguredSummary: React.ReactNode;
}) {
  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();
  const integrations = useQuery(udfs.listConfiguredSinks.default);

  const configuredIntegrations = integrations?.filter(
    (integration) =>
      "config" in integration &&
      "type" in integration.config &&
      integrationTypes.some((type) => integration.config.type === type),
  );

  const router = useRouter();

  const integrationStatus = !configuredIntegrations
    ? undefined
    : configuredIntegrations.length === 0
      ? "notConfigured"
      : configuredIntegrations[0].status.type === "active"
        ? "configured"
        : "error";

  const error =
    integrationStatus === "error"
      ? "The integration setup has failed."
      : undefined;

  const warning =
    integrations &&
    integrations.some((integration) =>
      integrationUsingLegacyFormat(integration.config),
    )
      ? `${integrations.length === 1 ? "This integration is" : "Some integrations are"} using a legacy event format. Re-configure the deprecated integration to update the event format.`
      : undefined;

  const integrationsPageLink = `${router.asPath}/settings/integrations`;

  const icons = (
    <div className="flex gap-2">
      {integrationTypes.map((kind, idx) => (
        <IntegrationIcon kind={kind} key={idx} />
      ))}
    </div>
  );

  return (
    <HealthCard
      title={title}
      error={error}
      warning={warning}
      size="sm"
      action={
        <>
          {configuredIntegrations &&
            configuredIntegrations.length === 0 &&
            icons}
          {configuredIntegrations && configuredIntegrations[0] && (
            <Button
              tip={`Go to ${integrationName(configuredIntegrations[0].config.type)}`}
              icon={<ExternalLinkIcon />}
              href={configToUrl(configuredIntegrations[0].config)}
              onClickOfAnchorLink={() => log("go to integration via health")}
              target="_blank"
              size="xs"
              inline
              variant="neutral"
            />
          )}
          <Button
            tip="Configure Integrations"
            icon={<GearIcon />}
            href={integrationsPageLink}
            onClickOfAnchorLink={() => log("configure integrations via health")}
            target="_blank"
            size="xs"
            inline
            variant="neutral"
          />
        </>
      }
    >
      <div className="flex h-full w-full items-center text-pretty px-2 pb-2">
        {integrationStatus === undefined ? (
          <Loading className="h-5 w-32" />
        ) : integrationStatus === "notConfigured" ? (
          <div className="animate-fadeInFromLoading">
            {notConfiguredSummary}
          </div>
        ) : integrationStatus === "configured" ? (
          configuredIntegrations !== undefined ? (
            <div className="flex animate-fadeInFromLoading items-start gap-2">
              {configuredIntegrations.map((integration, idx) => (
                <IntegrationIcon kind={integration.config.type} key={idx} />
              ))}
              {configuredIntegrations.length > 1 ? (
                <>
                  There are {configuredIntegrations.length} configured
                  integrations.
                </>
              ) : (
                <>
                  {integrationName(configuredIntegrations[0].config.type)} is
                  configured.
                </>
              )}
            </div>
          ) : (
            <Loading className="h-5 w-32" />
          )
        ) : (
          <span className="animate-fadeInFromLoading">
            Check your{" "}
            <Link
              href={integrationsPageLink}
              className="text-content-link hover:underline"
            >
              integration configuration
            </Link>
            .
          </span>
        )}
      </div>
    </HealthCard>
  );
}

function IntegrationIcon({ kind }: { kind: IntegrationType }) {
  return (
    <Tooltip
      tip={integrationName(kind)}
      className="shrink-0 animate-fadeInFromLoading cursor-default"
    >
      {integrationToLogo(kind, true).logo}{" "}
    </Tooltip>
  );
}
