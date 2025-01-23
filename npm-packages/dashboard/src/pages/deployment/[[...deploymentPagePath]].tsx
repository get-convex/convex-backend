import { useLastCreatedDeployment } from "hooks/useLastCreated";
import { useLastViewedDeployment } from "hooks/useLastViewed";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";
import { useEffect } from "react";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function RedirectToDeploymentPage() {
  const router = useRouter();

  const { deploymentPagePath, ...query } = router.query;
  const path = ((deploymentPagePath ?? []) as string[]).join("/");

  const [lastViewedDeploymentName] = useLastViewedDeployment();
  const lastCreatedDeployment = useLastCreatedDeployment();
  const deployment = lastViewedDeploymentName ?? lastCreatedDeployment?.name;

  useEffect(() => {
    void router.replace(
      deployment === undefined
        ? "/"
        : { pathname: `/d/${deployment}/${path}`, query },
    );
  });

  return null;
});
