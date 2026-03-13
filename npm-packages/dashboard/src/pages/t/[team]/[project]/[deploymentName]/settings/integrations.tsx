import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { IntegrationsView } from "@common/features/settings/components/integrations/IntegrationsView";
import { usePostHog } from "hooks/usePostHog";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

export { getServerSideProps } from "lib/ssr";

function IntegrationsWithAnalytics() {
  const { capture } = usePostHog();
  const { postHogIntegrations } = useLaunchDarkly();
  return (
    <IntegrationsView
      showPostHogIntegrations={postHogIntegrations}
      onAddedIntegration={(kind) => {
        capture("added_integration", { kind });
      }}
    />
  );
}

export default withAuthenticatedPage(IntegrationsWithAnalytics);
