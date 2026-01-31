import { useCurrentTeam } from "api/teams";
import { Sheet } from "@ui/Sheet";
import { useProvisionDeployment } from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useRouter } from "next/router";
import { useEffect, useRef } from "react";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { ProvisionDeploymentForm } from "./ProvisionDeploymentForm";

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

  const deploymentTypeLabel =
    deploymentType === "prod" ? "production" : "development";

  return showForm ? (
    projectId !== null ? (
      <ProvisionDeploymentForm
        projectId={projectId}
        projectURI={projectURI}
        deploymentType={deploymentType}
      />
    ) : null
  ) : (
    <div className="h-full bg-background-primary p-6">
      <Sheet className="mb-2 h-full overflow-hidden">
        <div className="flex flex-1 flex-col items-center justify-center">
          <div className="flex max-w-lg animate-fadeIn flex-col items-center">
            <h1 className="mx-2 mt-10 mb-8">
              Provisioning your{" "}
              <span className="font-semibold">{deploymentTypeLabel}</span>{" "}
              deployment...
              {projectId !== null ? (
                <ProvisionDeployment
                  projectId={projectId}
                  projectURI={projectURI}
                  deploymentType={deploymentType}
                />
              ) : null}
            </h1>
            <div className="w-full animate-fadeIn">
              <div className="h-4 rounded-sm bg-background-tertiary" />
            </div>
          </div>
        </div>
      </Sheet>
    </div>
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

  useEffect(() => {
    if (wasCalled.current) {
      return;
    }
    void (async () => {
      wasCalled.current = true;
      const { name } = await provisionDeployment({
        type: deploymentType,
      });
      void router.replace(`${projectURI}/${name}`);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}
