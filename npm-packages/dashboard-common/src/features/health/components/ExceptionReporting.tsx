import { IntegrationStatus } from "./IntegrationStatus";

export function ExceptionReporting() {
  return (
    <IntegrationStatus
      integrationTypes={["sentry"]}
      title="Exception Reporting"
      notConfiguredSummary="Get notified of function failures."
    />
  );
}
