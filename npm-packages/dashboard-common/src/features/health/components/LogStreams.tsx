import { IntegrationStatus } from "./IntegrationStatus";

export function LogStreams() {
  return (
    <IntegrationStatus
      integrationTypes={["axiom", "webhook", "datadog"]}
      title="Log Streams"
      notConfiguredSummary="Add persistence for function logs."
    />
  );
}
