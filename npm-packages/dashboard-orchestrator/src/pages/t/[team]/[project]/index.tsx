// Project root route — cloud doesn't expose a project home page; instead
// every project URL drops the user directly into a deployment. We mirror that
// by redirecting to the user's most-recently-viewed deployment, falling back
// to dev → prod → first preview. With `?provision=<kind>` we auto-create the
// requested deployment first; this is what the "Production/Development"
// rows in the deployment dropdown link to when their kind doesn't exist yet.

import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useRef, useState } from "react";
import { Spinner } from "@ui/Spinner";
import { Callout } from "@ui/Callout";
import {
  createDeployment,
  Deployment,
  listDeployments,
  listProjects,
  listTeams,
} from "../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../lib/config";

const LAST_VIEWED_DEPLOYMENT = (projectSlug: string) =>
  `orch-last-deployment-${projectSlug}`;

type DeployKind = "prod" | "dev" | "preview";

function pickDefault(
  deployments: Deployment[],
  projectSlug: string,
): Deployment | undefined {
  if (typeof window !== "undefined") {
    const last = window.localStorage.getItem(
      LAST_VIEWED_DEPLOYMENT(projectSlug),
    );
    if (last) {
      const match = deployments.find((d) => d.name === last);
      if (match) return match;
    }
  }
  return (
    deployments.find((d) => (d.kind ?? d.deploymentType) === "dev") ??
    deployments.find((d) => (d.kind ?? d.deploymentType) === "prod") ??
    deployments.find((d) => (d.kind ?? d.deploymentType) === "preview") ??
    deployments[0]
  );
}

export default function ProjectRedirectPage() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
  const provisionKind = router.query.provision as DeployKind | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();

  const { data: teams } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );
  const { data: projects } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );
  const project = useMemo(
    () => projects?.find((p) => p.slug === projectSlug),
    [projects, projectSlug],
  );
  const { data: deployments, mutate } = useSWR(
    project && token ? ["deployments", project.id, token] : null,
    () => listDeployments(url, token!, project!.id),
  );

  const [error, setError] = useState<string | null>(null);
  const provisioned = useRef(false);

  useEffect(() => {
    if (!team || !project || !deployments || !projectSlug || !token) return;

    // If the URL says ?provision=<kind> and no deployment of that kind
    // exists, mint one and then redirect to it. Each visit only triggers
    // provisioning once.
    if (provisionKind && !provisioned.current) {
      const existing = deployments.find(
        (d) => (d.kind ?? d.deploymentType) === provisionKind,
      );
      if (existing) {
        void router.replace(`/t/${team.slug}/${project.slug}/${existing.name}`);
        return;
      }
      provisioned.current = true;
      void (async () => {
        try {
          const fresh = await createDeployment(
            url,
            token,
            project.id,
            provisionKind,
          );
          await mutate();
          void router.replace(`/t/${team.slug}/${project.slug}/${fresh.name}`);
        } catch (err) {
          setError((err as Error).message);
        }
      })();
      return;
    }

    const target = pickDefault(deployments, projectSlug);
    if (target) {
      void router.replace(`/t/${team.slug}/${project.slug}/${target.name}`);
    }
    // If there's no deployment yet, fall through to the empty-state below.
  }, [
    team,
    project,
    deployments,
    projectSlug,
    token,
    url,
    provisionKind,
    mutate,
    router,
  ]);

  if (!team || !project || !token || !deployments) {
    return (
      <main className="flex flex-1 items-center justify-center">
        <Spinner className="ml-0" />
      </main>
    );
  }

  // Reachable when the project has zero deployments; we surface the kind
  // chooser inline rather than redirecting to a dedicated page.
  if (deployments.length === 0) {
    return (
      <main className="mx-auto flex w-full max-w-3xl flex-col gap-6 bg-background-primary p-8">
        <div className="flex flex-col gap-1">
          {/* eslint-disable-next-line no-restricted-syntax -- text-2xl IS the heading style on an h1 */}
          <h1 className="text-2xl font-semibold text-content-primary">
            {project.name}
          </h1>
          <p className="text-sm text-content-secondary">
            This project has no deployments yet. Provision one to get started.
          </p>
        </div>
        {error && <Callout variant="error">{error}</Callout>}
        <div className="flex gap-2">
          {(["prod", "dev", "preview"] as const).map((kind) => (
            <ProvisionButton
              key={kind}
              kind={kind}
              teamSlug={team.slug}
              projectSlug={project.slug}
            />
          ))}
        </div>
      </main>
    );
  }

  // Visible briefly while the redirect lands.
  return (
    <main className="flex flex-1 items-center justify-center">
      <Spinner className="ml-0" />
    </main>
  );
}

function ProvisionButton({
  kind,
  teamSlug,
  projectSlug,
}: {
  kind: DeployKind;
  teamSlug: string;
  projectSlug: string;
}) {
  const router = useRouter();
  const label =
    kind === "prod" ? "Production" : kind === "dev" ? "Development" : "Preview";
  return (
    // eslint-disable-next-line react/forbid-elements -- inline provisioner trigger styled as a card, intentional plain <button>
    <button
      type="button"
      onClick={() =>
        router.replace(`/t/${teamSlug}/${projectSlug}?provision=${kind}`)
      }
      className="rounded-md border border-border-transparent bg-background-secondary px-4 py-2 text-sm text-content-primary hover:bg-background-tertiary"
    >
      + {label}
    </button>
  );
}
