import { useCurrentTeam } from "api/teams";
import { Sheet } from "dashboard-common/elements/Sheet";
import { useProvisionDeployment } from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useRouter } from "next/router";
import { useEffect, useRef } from "react";

export function ProvisionProductionDeploymentPage() {
  const router = useRouter();
  const projectSlug = router.query.project as string;
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const projectId = project?.id ?? null;
  const projectURI = `/t/${team?.slug}/${projectSlug}`;

  return (
    <div className="h-full bg-background-primary p-6">
      <Sheet className="mb-2 h-full overflow-hidden">
        <div className="flex flex-1 flex-col items-center justify-center">
          <div className="flex max-w-lg animate-fadeIn flex-col items-center">
            <h1 className="mx-2 mb-8 mt-10">
              Provisioning your{" "}
              <span className="font-semibold">production</span> deployment...
              {projectId !== null ? (
                <ProvisionDeployment
                  projectId={projectId}
                  projectURI={projectURI}
                />
              ) : null}
            </h1>
            <div className="w-full animate-fadeIn">
              <div className="h-4 rounded bg-background-tertiary" />
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
}: {
  projectId: number;
  projectURI: string;
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
      const { deploymentName } = await provisionDeployment({
        deploymentType: "prod",
      });
      void router.replace(`${projectURI}/${deploymentName}`);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}
