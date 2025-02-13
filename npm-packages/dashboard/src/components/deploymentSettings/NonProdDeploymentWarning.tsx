import { ReactNode } from "react";
import { useRouter } from "next/router";
import { useDeployments } from "api/deployments";
import { useProjects } from "api/projects";
import { useCurrentTeam } from "api/teams";
import Link from "next/link";
import { DeploymentType as DeploymentTypeType } from "generatedApi";
import { Sheet } from "dashboard-common/elements/Sheet";
import { DeploymentType } from "dashboard-common/features/settings/components/DeploymentUrl";

export function NonProdDeploymentWarning({
  deploymentType,
  children,
}: {
  deploymentType: Exclude<DeploymentTypeType, "prod">;
  children: ReactNode;
}) {
  const router = useRouter();
  const projectSlug = router?.query.project as string;
  const team = useCurrentTeam();
  const projects = useProjects(team?.id);

  const selectedProject =
    projects && projects.find((project) => project.slug === projectSlug);
  const { deployments } = useDeployments(selectedProject?.id) || [];
  const prod = deployments?.find((d) => d.deploymentType === "prod");
  const projectsURI = `/t/${team?.slug}/${projectSlug}`;
  const prodUrl = `${projectsURI}/${prod?.name}/settings`;

  const explanation =
    deploymentType === "dev" ? (
      <>
        This personal <DeploymentType deploymentType="dev" /> Convex deployment
        also has deployment credentials. Edits you make to functions in your
        editor sync automatically, but it is also possible to deploy to it from
        somewhere else using a deploy key.
      </>
    ) : (
      <>
        This <DeploymentType deploymentType="preview" /> Convex deployment also
        has deployment credentials, which are typically used in a Vercel or
        Netlify build step to sync your functions. But it is also possible to
        deploy to it from somewhere else using a deploy key.
      </>
    );

  return (
    <Sheet padding={false}>
      <div className="p-6 text-sm">
        <h3 className="mb-4">Deployment URL and Deploy Key</h3>
        {prod ? (
          <p className="mb-4 text-content-primary">
            <Link
              href={prodUrl}
              passHref
              className="text-content-link hover:underline"
            >
              Go to this project's <DeploymentType deploymentType="prod" />{" "}
              deployment
            </Link>{" "}
            for credentials used to deploy this project.
          </p>
        ) : (
          <p className="mb-4 text-content-primary">
            <Link
              href={`${projectsURI}/production`}
              passHref
              className="text-content-link hover:underline"
            >
              Create a <DeploymentType deploymentType="prod" /> deployment
            </Link>{" "}
            to deploy this project to a live environment.
          </p>
        )}
        <p className="mb-4 text-content-primary">{explanation}</p>
      </div>
      {/* TODO: Replace with disclosure */}
      {/* eslint-disable-next-line react/forbid-elements */}
      <details className="border-t text-sm">
        {/* eslint-disable-next-line react/forbid-elements */}
        <summary className="cursor-pointer p-6 text-content-primary">
          Show <DeploymentType deploymentType={deploymentType} /> credentials
        </summary>
        <div className="flex flex-col gap-4">{children}</div>
      </details>
    </Sheet>
  );
}
