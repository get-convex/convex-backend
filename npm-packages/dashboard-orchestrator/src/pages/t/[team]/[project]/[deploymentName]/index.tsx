import { HealthView } from "@common/features/health/components/HealthView";
import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export default function HealthPage() {
  const { useCurrentTeam, useCurrentProject, useCurrentDeployment } =
    useContext(DeploymentInfoContext);
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();

  return (
    <HealthView
      header={<h3 className="sticky top-0 mx-6 pt-4 pb-2">Health</h3>}
      PagesWrapper={({ children }) => (
        <div className="flex min-h-0 grow">{children}</div>
      )}
      PageWrapper={({ children }) => (
        <div className="scrollbar max-w-full shrink-0 grow overflow-y-auto px-6 pb-4">
          {children}
        </div>
      )}
      deployment={deployment}
      teamSlug={team?.slug}
      projectSlug={project?.slug}
    />
  );
}
