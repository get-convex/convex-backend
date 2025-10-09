import { useDeployments } from "api/deployments";
import { useCurrentTeam } from "api/teams";
import { useProjects } from "api/projects";
import { useProfile } from "api/profile";
import { useRouter } from "next/router";
import { Loading } from "@ui/Loading";
import { useEffect } from "react";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function RedirectToDeployment() {
  const router = useRouter();
  const currentTeam = useCurrentTeam();
  const projectSlug =
    typeof router.query.project === "string" ? router.query.project : undefined;
  const projects = useProjects(currentTeam?.id);
  const currentProject =
    projects?.find((project) => project.slug === projectSlug) ?? undefined;
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
    if (shownDeploymentName) {
      void router.replace(
        `/t/${currentTeam!.slug}/${
          currentProject!.slug
        }/${shownDeploymentName}`,
      );
    } else if (deployments) {
      // Couldn't find a deployment to display
      void router.replace("/404");
    }
  }, [deployments, currentTeam, currentProject, shownDeploymentName, router]);

  return <Loading />;
});
