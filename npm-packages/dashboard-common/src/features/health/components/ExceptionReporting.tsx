import { IntegrationStatus } from "features/health/components/IntegrationStatus";

export function ExceptionReporting() {
  return (
    <IntegrationStatus
      integrationTypes={["sentry"]}
      title="Exception Reporting"
      notConfiguredSummary="Get notified of function failures."
    />
  );
}
