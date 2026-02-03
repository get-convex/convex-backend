import { useCurrentTeam } from "api/teams";
import { useProvisionDeployment } from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useRouter } from "next/router";
import { useRef } from "react";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useIsomorphicLayoutEffect } from "react-use";
import { ProvisionDeploymentForm } from "./ProvisionDeploymentForm";
import { ProvisioningLoading } from "./ProvisioningLoading";

export function ProvisionDeploymentPage({
  deploymentType,
}: {
  deploymentType: "prod" | "dev";
}) {
  const router = useRouter();
  const projectSlug = router.query.project as string;
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const projectId = project?.id ?? null;
  const projectURI = `/t/${team?.slug}/${projectSlug}`;
  const flags = useLaunchDarkly();
  const showForm = flags.deploymentRegion;

  return showForm ? (
    projectId !== null ? (
      <ProvisionDeploymentForm
        projectId={projectId}
        projectURI={projectURI}
        deploymentType={deploymentType}
      />
    ) : null
  ) : (
    <>
      {projectId !== null && (
        <ProvisionDeployment
          projectId={projectId}
          projectURI={projectURI}
          deploymentType={deploymentType}
        />
      )}
      <ProvisioningLoading deploymentType={deploymentType} />
    </>
  );
}

function ProvisionDeployment({
  projectId,
  projectURI,
  deploymentType,
}: {
  projectId: number;
  projectURI: string;
  deploymentType: "prod" | "dev";
}) {
  const router = useRouter();
  const provisionDeployment = useProvisionDeployment(projectId);

  // We know that this component will get unmounted
  // after the successful provisioning, so we only
  // care about the useEffect ever running once!
  //
  // This avoids calling the API twice, even in React StrictMode.
  const wasCalled = useRef(false);

  // Using useIsomorphicLayoutEffect instead of useEffect
  // to avoid a weird bug where the effect would run twice
  // when the page is accessed from a Next.js <Link />
  useIsomorphicLayoutEffect(() => {
    if (wasCalled.current) {
      return;
    }
    wasCalled.current = true;
    void (async () => {
      const { name } = await provisionDeployment({
        type: deploymentType,
      });
      void router.replace(`${projectURI}/${name}`);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}
