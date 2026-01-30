import { useDeployments } from "api/deployments";
import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useProfile } from "api/profile";
import { useRouter } from "next/router";
import { Loading } from "@ui/Loading";
import { useEffect } from "react";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { PROVISION_DEV_PAGE_NAME } from "@common/lib/deploymentContext";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function RedirectToDeployment() {
  const router = useRouter();
  const currentTeam = useCurrentTeam();
  const currentProject = useCurrentProject();
  const { deployments } = useDeployments(currentProject?.id);
  const member = useProfile();
  const prodDeployment = deployments?.find((d) => d.deploymentType === "prod");
  const devDeployment = deployments?.find(
    (d) => d.deploymentType === "dev" && d.creator === member?.id,
  );
  const anyDeployment = deployments?.[0];
  const shownDeployment = devDeployment ?? prodDeployment ?? anyDeployment;
  const shownDeploymentName = shownDeployment?.name;

  useEffect(() => {
    if (!currentTeam || !currentProject) {
      // Still loading?
      // (Normally this shouldn’t happen on the Convex Cloud dashboard because we use SSR)
      return;
    }

    if (shownDeploymentName) {
      void router.replace(
        `/t/${currentTeam.slug}/${currentProject.slug}/${shownDeploymentName}`,
      );
    } else if (deployments) {
      // No deployments found → go to the page that provisions the default dev deployment
      void router.replace(
        `/t/${currentTeam.slug}/${currentProject.slug}/${PROVISION_DEV_PAGE_NAME}`,
      );
    }
  }, [deployments, currentTeam, currentProject, shownDeploymentName, router]);

  return <Loading />;
});
