import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { Sheet } from "@ui/Sheet";
import { useRouter } from "next/router";
import useSWR from "swr";
import { useMemo } from "react";
import {
  listDeployments,
  listProjects,
  listTeams,
} from "../../../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../../../lib/config";
import { CustomDomainsCard } from "../../../../../../components/CustomDomainsCard";
import { DnsCredentialsCard } from "../../../../../../components/DnsCredentialsCard";

// The route is keyed by deployment *name*, but the custom-domains API is
// keyed by deployment id, so resolve team -> project -> deployment to get it.
export default function CustomDomains() {
  const router = useRouter();
  const query = router.query as {
    team?: string;
    project?: string;
    deploymentName?: string;
  };
  const { team: teamSlug, project: projectSlug, deploymentName } = query;
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
    token && team ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );
  const project = useMemo(
    () => projects?.find((p) => p.slug === projectSlug),
    [projects, projectSlug],
  );

  const { data: deployments } = useSWR(
    token && project ? ["deployments", project.id, token] : null,
    () => listDeployments(url, token!, project!.id),
  );
  const deployment = useMemo(
    () => deployments?.find((d) => d.name === deploymentName),
    [deployments, deploymentName],
  );

  return (
    <DeploymentSettingsLayout page="custom-domains">
      {deployment === undefined ? (
        <Sheet>
          <h3>Custom Domains</h3>
          <p className="mt-2 text-sm text-content-secondary">
            {deployments === undefined
              ? "Loading…"
              : `No deployment named ${deploymentName ?? ""} in this project.`}
          </p>
        </Sheet>
      ) : (
        <div className="flex flex-col gap-6">
          <CustomDomainsCard
            deploymentId={deployment.id}
            deploymentName={deployment.name}
            teamId={team?.id}
          />
          {/* The domains card points here when dns-01 is selected without a
              credential. Without this the hint would be a dead end. */}
          <DnsCredentialsCard teamId={team?.id} />
        </div>
      )}
    </DeploymentSettingsLayout>
  );
}
