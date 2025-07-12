import classNames from "classnames";
import { ExportIntegrationType } from "system-udfs/convex/_system/frontend/common";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Modal } from "@ui/Modal";
import {
  IntegrationUnavailableReason,
  LogIntegration,
  ExceptionReportingIntegration,
  integrationToLogo,
  STREAMING_EXPORT_DESCRIPTION,
  LOG_STREAMS_DESCRIPTION,
} from "@common/lib/integrationHelpers";
import { useState, ReactNode, useCallback } from "react";
import { IntegrationTitle } from "./IntegrationTitle";
import { IntegrationOverflowMenu } from "./IntegrationOverflowMenu";
import { IntegrationStatus } from "./IntegrationStatus";
import { AxiomConfigurationForm } from "./AxiomConfigurationForm";
import { DatadogConfigurationForm } from "./DatadogConfigurationForm";
import { SentryConfigurationForm } from "./SentryConfigurationForm";
import { WebhookConfigurationForm } from "./WebhookConfigurationForm";

export type PanelCardProps = {
  className?: string;
  integration:
    | LogIntegration
    | ExceptionReportingIntegration
    | { kind: ExportIntegrationType };
  unavailableReason: IntegrationUnavailableReason | null;
};

export function PanelCard({
  className,
  integration,
  unavailableReason,
}: PanelCardProps) {
  const classes = classNames(
    "py-3 px-4",
    "items-center gap-2.5 transition-colors rounded-sm text-sm font-medium",
    "border",
    className,
  );
  const { logo } = integrationToLogo(integration.kind);

  const [modalState, setModalState] = useState<{
    showing: boolean;
    content?: ReactNode;
  }>({
    showing: false,
    content: undefined,
  });

  const closeModal = useCallback(() => {
    setModalState({
      showing: false,
      content: undefined,
    });
  }, [setModalState]);
  () => {};

  return (
    <div className={classes}>
      {(integration.kind === "airbyte" || integration.kind === "fivetran") && (
        <div className="flex flex-wrap items-center justify-between gap-2">
          <IntegrationTitle
            logo={logo}
            integrationKind={integration.kind}
            description={STREAMING_EXPORT_DESCRIPTION}
          />
          <div className="ml-auto">
            {unavailableReason === null && (
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
          {modalState.content}
          <IntegrationTitle
            logo={logo}
            integrationKind={integration.kind}
            description={LOG_STREAMS_DESCRIPTION}
          />
          <div className="flex items-center gap-4">
            <IntegrationStatus integration={integration} />
            {unavailableReason === null && (
              <IntegrationOverflowMenu
                integration={integration}
                onConfigure={() =>
                  setModalState({
                    showing: true,
                    content:
                      renderModal && renderModal(integration, closeModal),
                  })
                }
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
      const _typeCheck: never = kind;
      return "";
    }
  }
}

function renderModal(
  integration: LogIntegration | ExceptionReportingIntegration,
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
    default: {
      const _typeCheck: never = integration;
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
