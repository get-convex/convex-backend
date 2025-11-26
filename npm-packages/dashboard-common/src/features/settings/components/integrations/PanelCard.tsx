import classNames from "classnames";
import { ExportIntegrationType } from "system-udfs/convex/_system/frontend/common";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Modal } from "@ui/Modal";
import { Tooltip } from "@ui/Tooltip";
import Link from "next/link";
import {
  IntegrationUnavailableReason,
  LogIntegration,
  ExceptionReportingIntegration,
  AuthIntegration,
  integrationToLogo,
  STREAMING_EXPORT_DESCRIPTION,
  LOG_STREAMS_DESCRIPTION,
  AUTHENTICATION_DESCRIPTION,
} from "@common/lib/integrationHelpers";
import { useState, useCallback } from "react";
import { IntegrationTitle } from "./IntegrationTitle";
import { IntegrationOverflowMenu } from "./IntegrationOverflowMenu";
import { IntegrationStatus } from "./IntegrationStatus";
import { AxiomConfigurationForm } from "./AxiomConfigurationForm";
import { DatadogConfigurationForm } from "./DatadogConfigurationForm";
import { SentryConfigurationForm } from "./SentryConfigurationForm";
import { WebhookConfigurationForm } from "./WebhookConfigurationForm";
import { WorkOSConfigurationForm } from "./WorkOSConfigurationForm";
import { WorkOSIntegrationStatus } from "./WorkOSIntegrationStatus";
import { WorkOSIntegrationOverflowMenu } from "./WorkOSIntegrationOverflowMenu";

export type PanelCardProps = {
  className?: string;
  integration:
    | LogIntegration
    | ExceptionReportingIntegration
    | AuthIntegration
    | { kind: ExportIntegrationType };
  unavailableReason: IntegrationUnavailableReason | null;
  teamSlug?: string;
};

function ProBadge({ teamSlug }: { teamSlug?: string }) {
  const badge = (
    <span className="cursor-pointer rounded-sm bg-util-accent px-1.5 py-0.5 text-xs font-semibold tracking-wider text-white uppercase">
      Pro
    </span>
  );

  if (!teamSlug) {
    return <Tooltip tip="Only available on the Pro plan">{badge}</Tooltip>;
  }

  return (
    <Tooltip tip="Only available on the Pro plan">
      <Link href={`/${teamSlug}/settings/billing`}>{badge}</Link>
    </Tooltip>
  );
}

export function PanelCard({
  className,
  integration,
  unavailableReason,
  teamSlug,
}: PanelCardProps) {
  const classes = classNames(
    "py-3 px-4",
    "items-center gap-2.5 transition-colors rounded-sm text-sm font-medium",
    "border",
    className,
  );
  const { logo } = integrationToLogo(integration.kind);

  const [isModalOpen, setIsModalOpen] = useState(false);

  const closeModal = useCallback(() => {
    setIsModalOpen(false);
  }, []);

  return (
    <div className={classes}>
      {integration.kind === "workos" && (
        <div className="flex flex-wrap items-center justify-between gap-2">
          {isModalOpen && renderModal(integration, closeModal)}
          <IntegrationTitle
            logo={logo}
            integrationKind={integration.kind}
            description={AUTHENTICATION_DESCRIPTION}
          />
          <div className="flex items-center gap-4">
            <WorkOSIntegrationStatus integration={integration} />
            <WorkOSIntegrationOverflowMenu
              integration={integration}
              onConfigure={() => setIsModalOpen(true)}
            />
          </div>
        </div>
      )}
      {(integration.kind === "airbyte" || integration.kind === "fivetran") && (
        <div className="flex flex-wrap items-center justify-between gap-2">
          <IntegrationTitle
            logo={logo}
            integrationKind={integration.kind}
            description={STREAMING_EXPORT_DESCRIPTION}
          />
          <div className="ml-auto">
            {unavailableReason === "MissingEntitlement" ? (
              <ProBadge teamSlug={teamSlug} />
            ) : (
              <Button
                href={exportSetupLink(integration.kind)}
                target="_blank"
                className="flex items-center gap-2"
                inline
                variant="neutral"
              >
                <div>Get Started</div>
                <ExternalLinkIcon />
              </Button>
            )}
          </div>
        </div>
      )}
      {(integration.kind === "sentry" ||
        integration.kind === "axiom" ||
        integration.kind === "datadog" ||
        integration.kind === "webhook") && (
        <div className="flex flex-wrap items-center justify-between gap-2">
          {isModalOpen && renderModal(integration, closeModal)}
          <IntegrationTitle
            logo={logo}
            integrationKind={integration.kind}
            description={LOG_STREAMS_DESCRIPTION}
          />
          <div className="flex items-center gap-4">
            <IntegrationStatus integration={integration} />
            {unavailableReason === "MissingEntitlement" ? (
              <ProBadge teamSlug={teamSlug} />
            ) : (
              <IntegrationOverflowMenu
                integration={integration}
                onConfigure={() => setIsModalOpen(true)}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function exportSetupLink(kind: ExportIntegrationType): string {
  switch (kind) {
    case "airbyte":
      return "https://docs.airbyte.com/integrations/sources/convex";
    case "fivetran":
      return "https://fivetran.com/integrations/convex";
    default: {
      kind satisfies never;
      return "";
    }
  }
}

function renderModal(
  integration: LogIntegration | ExceptionReportingIntegration | AuthIntegration,
  closeModal: () => void,
) {
  switch (integration.kind) {
    case "datadog": {
      return (
        <LogIntegrationModal
          closeModal={closeModal}
          title="Configure Datadog"
          description="Configure your Convex deployment to route logs to Datadog to persist your logs and enable custom log queries and dashboards."
        >
          <DatadogConfigurationForm
            existingConfig={integration.existing?.config ?? null}
            onClose={closeModal}
          />
        </LogIntegrationModal>
      );
    }
    case "axiom":
      return (
        <LogIntegrationModal
          closeModal={closeModal}
          title="Configure Axiom"
          description="Configure your Convex deployment to route logs to Axiom to persist your logs and enable custom log queries and dashboards."
        >
          <AxiomConfigurationForm
            existingConfig={integration.existing?.config ?? null}
            onClose={closeModal}
          />
        </LogIntegrationModal>
      );
    case "webhook":
      return (
        <LogIntegrationModal
          closeModal={closeModal}
          title="Configure Webhook"
          description="Configure your Convex deployment to send JSON logs via POST requests."
        >
          <WebhookConfigurationForm
            onClose={closeModal}
            existingIntegration={integration.existing?.config ?? null}
          />
        </LogIntegrationModal>
      );

    case "sentry":
      return (
        <Modal onClose={closeModal} title="Configure Sentry">
          <div className="flex flex-col gap-4">
            <div className="max-w-prose text-xs text-pretty text-content-secondary">
              Configure your Convex deployment to route function execution
              exceptions to Sentry for visibility.
            </div>
            <SentryConfigurationForm
              existingConfig={integration.existing?.config ?? null}
              onClose={closeModal}
            />
          </div>
        </Modal>
      );
    case "workos":
      return (
        <Modal onClose={closeModal} title="WorkOS AuthKit Environment">
          <WorkOSConfigurationForm />
        </Modal>
      );
    default: {
      integration satisfies never;
      return null;
    }
  }
}

function LogIntegrationModal({
  title,
  description,
  closeModal,
  children,
}: {
  title: string;
  description: string;
  closeModal: () => void;
  children: React.ReactNode;
}) {
  return (
    <Modal onClose={closeModal} title={title}>
      <div className="flex flex-col gap-4">
        <div className="max-w-prose text-xs text-pretty text-content-secondary">
          {description}
        </div>
        {children}
      </div>
    </Modal>
  );
}
