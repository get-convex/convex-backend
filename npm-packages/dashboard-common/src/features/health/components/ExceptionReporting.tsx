import { IntegrationStatus } from "@common/features/health/components/IntegrationStatus";

export function ExceptionReporting() {
  return (
    <IntegrationStatus
      integrationTypes={["sentry"]}
      title="Exception Reporting"
      notConfiguredSummary="Get notified of function failures."
    />
  );
}
