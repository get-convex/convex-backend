import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { PauseDeploymentView } from "@common/features/settings/components/PauseDeploymentView";
import { usePostHog } from "hooks/usePostHog";

export { getServerSideProps } from "lib/ssr";

function PauseDeploymentWithAnalytics() {
  const { capture } = usePostHog();
  return (
    <PauseDeploymentView
      onPausedDeployment={() => {
        capture("paused_deployment");
      }}
    />
  );
}

export default withAuthenticatedPage(PauseDeploymentWithAnalytics);
