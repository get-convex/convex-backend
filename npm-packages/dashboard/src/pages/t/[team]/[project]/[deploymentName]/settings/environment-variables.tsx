import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { EnvironmentVariablesView } from "@common/features/settings/components/EnvironmentVariablesView";
import { usePostHog } from "hooks/usePostHog";

export { getServerSideProps } from "lib/ssr";

function EnvironmentVariablesWithAnalytics() {
  const { capture } = usePostHog();
  return (
    <EnvironmentVariablesView
      onEnvironmentVariablesAdded={(count) =>
        capture("added_environment_variables", { count })
      }
    />
  );
}

export default withAuthenticatedPage(EnvironmentVariablesWithAnalytics);
