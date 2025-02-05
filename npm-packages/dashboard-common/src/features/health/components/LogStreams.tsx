import { IntegrationStatus } from "@common/features/health/components/IntegrationStatus";

export function LogStreams() {
  return (
    <IntegrationStatus
      integrationTypes={["axiom", "webhook", "datadog"]}
      title="Log Streams"
      notConfiguredSummary="Add persistence for function logs."
    />
  );
}
