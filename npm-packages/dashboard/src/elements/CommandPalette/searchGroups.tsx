import { Command } from "cmdk";
import { PlusIcon } from "@radix-ui/react-icons";
import { useCurrentTeam } from "api/teams";
import { useInfiniteProjects } from "api/projects";
import { usePaginatedDeployments } from "api/deployments";
import type { ProjectDetails, TeamResponse } from "generatedApi";
import { NavigationDestination, REMOTE_VALUE_PREFIX } from "./navigation";
import { DeploymentItem, LoadingSignal, ProjectItem } from "./items";
import { PalettePage } from "./pages";

// The drilled-into "Switch Project" page: the full, searchable project list.
export function SwitchProjectCommands({
  search,
  onNavigate,
  pushPage,
}: {
  search: string;
  onNavigate: (to: NavigationDestination) => void;
  pushPage: (page: PalettePage) => void;
}) {
  const team = useCurrentTeam();

  if (!team) {
    return <LoadingSignal />;
  }

  return (
    <ProjectSearchGroup
      team={team}
      search={search}
      onNavigate={onNavigate}
      pushPage={pushPage}
      full
    />
  );
}

export function ProjectSearchGroup({
  team,
  search,
  onNavigate,
  pushPage,
  full = false,
}: {
  team: TeamResponse;
  search: string;
  onNavigate: (to: NavigationDestination) => void;
  pushPage: (page: PalettePage) => void;
  // Show the whole (paginated) list rather than a root-page teaser.
  full?: boolean;
}) {
  const { projects, isLoading, hasMore, loadMore, debouncedQuery } =
    useInfiniteProjects(team.id, search, false);
  const trimmed = search.trim();
  const stale = isLoading || debouncedQuery.trim() !== trimmed;

  // With no search, show a short list so the root page stays scannable;
  // server-side search takes over as soon as the user types.
  const shown = full || trimmed ? projects : projects?.slice(0, 5);

  if (stale) {
    return (
      <Command.Group heading={`${team.name || team.slug} · Projects`}>
        <LoadingSignal />
      </Command.Group>
    );
  }

  return (
    <Command.Group heading={`${team.name || team.slug} · Projects`}>
      {shown?.map((candidate) => (
        <ProjectItem
          key={candidate.id}
          project={candidate}
          teamSlug={team.slug}
          teamName={team.name}
          onNavigate={onNavigate}
          onDrill={() => pushPage({ type: "project", project: candidate })}
        />
      ))}
      {(full || trimmed) && hasMore && (
        <Command.Item
          value={`${REMOTE_VALUE_PREFIX}projects-load-more`}
          className="animate-fadeInFromLoading"
          onSelect={loadMore}
        >
          <PlusIcon className="text-content-secondary" />
          Load more projects
        </Command.Item>
      )}
    </Command.Group>
  );
}

export function DeploymentSearchGroup({
  team,
  project,
  search,
  onNavigate,
  pushPage,
}: {
  team: TeamResponse;
  // When set, the search is scoped to this project's deployments.
  project: ProjectDetails | undefined;
  search: string;
  onNavigate: (to: NavigationDestination) => void;
  pushPage: (page: PalettePage) => void;
}) {
  const q = search.trim();
  const result = usePaginatedDeployments(team.id, {
    q,
    projectId: project?.id,
    // These rows bypass cmdk's filter (paletteFilter always keeps remote
    // items), so clear to a loading row rather than keeping the prior query's
    // deployments visible while a new query loads.
    keepPreviousData: false,
  });
  const deployments = (result?.items ?? [])
    .filter((d) => d.kind === "cloud")
    .slice(0, 8);
  const stale = result === undefined || result.isLoading;

  return (
    <Command.Group
      heading={`${(project ? project.name || project.slug : undefined) ?? team.name ?? team.slug} · Deployments`}
    >
      {stale ? (
        <LoadingSignal />
      ) : (
        deployments.map((deployment) => (
          <DeploymentItem
            key={deployment.name}
            deployment={deployment}
            teamSlug={team.slug}
            projectSlug={project?.slug}
            remote
            // In a project-scoped search, the project would be the same on
            // every row.
            showProject={project === undefined}
            onNavigate={onNavigate}
            onDrill={() =>
              pushPage({
                type: "deployment",
                deployment,
                projectSlug: project?.slug,
              })
            }
          />
        ))
      )}
    </Command.Group>
  );
}
