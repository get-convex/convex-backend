import { useRouter } from "next/router";
import useSWR from "swr";
import { useEffect, useMemo, useState } from "react";
import { Sheet } from "@ui/Sheet";
import { TeamSettingsLayout } from "../../../../components/TeamSettingsLayout";
import {
  listProjects,
  listTeams,
  Project,
  Team,
} from "../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../lib/config";

const RESOURCES = [
  {
    label: "Function Calls",
    note: "All function executions across deployments",
  },
  { label: "Action Compute", note: "GB-hours used by long-running actions" },
  { label: "Database Storage", note: "Bytes stored in document tables" },
  { label: "Database I/O", note: "Bytes read/written from document tables" },
  { label: "File Storage", note: "Bytes stored in file storage" },
  { label: "Data Egress", note: "Bytes sent over the network" },
  { label: "Search Storage", note: "Bytes indexed in search indexes" },
  { label: "Search Queries", note: "Search queries served, in qGB" },
  { label: "Deployments", note: "Total provisioned deployments" },
];

export default function TeamUsagePage() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectFilter = router.query.projectSlug as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const { data: teams } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team: Team | undefined = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );
  const { data: projects } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );

  if (!mounted || !team || !token) return null;

  const focusedProject = projectFilter
    ? (projects ?? []).find((p) => p.slug === projectFilter)
    : undefined;

  return (
    <TeamSettingsLayout page="usage" title="Usage">
      <Sheet>
        <h3 className="mb-2 text-base font-semibold">
          {focusedProject
            ? `Usage for ${focusedProject.name}`
            : "Self-hosted unlimited"}
        </h3>
        <p className="max-w-prose text-sm text-content-secondary">
          {focusedProject ? (
            <>
              Showing the team's usage view scoped to{" "}
              <code className="rounded-sm bg-background-tertiary px-1 font-mono">
                {focusedProject.slug}
              </code>
              . The orchestrator does not enforce per-resource quotas the way
              Convex Cloud does — every resource is unmetered.
            </>
          ) : (
            <>
              The orchestrator does not enforce per-resource quotas the way
              Convex Cloud does. The list below is for parity with the cloud
              dashboard's surface — every resource is unmetered and the limits
              are informational.
            </>
          )}
        </p>
      </Sheet>
      <Sheet>
        <h3 className="mb-4 text-base font-semibold">Resources</h3>
        <table className="w-full text-sm">
          <thead className="text-left text-xs text-content-secondary uppercase">
            <tr>
              <th className="pb-2">Resource</th>
              <th className="pb-2">Included</th>
              <th className="pb-2">On-demand</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-transparent">
            {RESOURCES.map((r) => (
              <tr key={r.label}>
                <td className="py-3">
                  <div className="font-medium text-content-primary">
                    {r.label}
                  </div>
                  <div className="text-xs text-content-secondary">{r.note}</div>
                </td>
                <td className="py-3 text-content-secondary">Unlimited</td>
                <td className="py-3 text-content-secondary">—</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Sheet>
      <Sheet>
        <h3 className="mb-4 text-base font-semibold">Projects in this team</h3>
        <ul className="divide-y divide-border-transparent">
          {(projects ?? []).map((p: Project) => (
            <li
              key={p.id}
              className="flex items-center justify-between gap-3 py-3"
            >
              <div className="min-w-0">
                <div className="truncate text-sm font-medium text-content-primary">
                  {p.name}
                </div>
                <div className="truncate text-xs text-content-secondary">
                  {p.slug}
                </div>
              </div>
              <span className="text-xs text-content-secondary">Unlimited</span>
            </li>
          ))}
          {(projects ?? []).length === 0 && (
            <li className="py-3 text-sm text-content-secondary">
              No projects yet.
            </li>
          )}
        </ul>
      </Sheet>
    </TeamSettingsLayout>
  );
}
