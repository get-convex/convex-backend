import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { FunctionsView } from "@common/features/functions/components/FunctionsView";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

function FunctionsPage() {
  const { subscriptionInvalidationsChart } = useLaunchDarkly();
  return (
    <FunctionsView
      showSubscriptionInvalidations={subscriptionInvalidationsChart}
    />
  );
}

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(FunctionsPage);
