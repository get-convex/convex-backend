import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";
import { useEffect } from "react";

export { getServerSideProps } from "lib/ssr";

function PauseDeploymentRedirect() {
  const router = useRouter();

  useEffect(() => {
    const { team, project, deploymentName } = router.query;
    if (team && project && deploymentName) {
      void router.replace(
        `/t/${team}/${project}/${deploymentName}/settings#pause-deployment`,
      );
    }
  }, [router]);

  return null;
}

export default withAuthenticatedPage(PauseDeploymentRedirect);
