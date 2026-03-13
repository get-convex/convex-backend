import {
  ExternalLinkIcon,
  GridIcon,
  ListBulletIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { ProjectCard } from "components/projects/ProjectCard";
import {
  DeploymentList,
  DeploymentToolbar,
  useDeploymentsWithFilters,
} from "components/deployments/DeploymentList";
import { usePaginatedProjects } from "api/projects";
import {
  useProjectsPageSize,
  PROJECT_PAGE_SIZES,
} from "hooks/useProjectsPageSize";
import { useCurrentTeam } from "api/teams";
import { useTeamOrbSubscription } from "api/billing";
import { useReferralState } from "api/referrals";
import { ProjectDetails, TeamResponse } from "generatedApi";
import { ReferralsBanner } from "components/referral/ReferralsBanner";
import { useCreateProjectModal } from "hooks/useCreateProjectModal";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import Head from "next/head";
import { useState, useEffect } from "react";
import { useDebounce } from "react-use";
import { cn } from "@ui/cn";
import { SegmentedControl } from "@ui/SegmentedControl";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { EmptySection } from "@common/elements/EmptySection";
import { OpenInVercel } from "components/OpenInVercel";
import { LoadingLogo } from "@ui/Loading";
import { PaginationControls } from "elements/PaginationControls";
import { useRouter } from "next/router";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => {
  const team = useCurrentTeam();
  const router = useRouter();
  const { deploymentList: deploymentListEnabled } = useLaunchDarkly();
  const referralState = useReferralState(team?.id);
  const { subscription } = useTeamOrbSubscription(team?.id);
  const isFreePlan =
    subscription === undefined ? undefined : subscription === null;
  const [prefersReferralsBannerHidden, setPrefersReferralsBannerHidden] =
    useGlobalLocalStorage("prefersReferralsBannerHidden", false);

  const viewFromQuery = (router.query.view as string | undefined) ?? "projects";
  const currentView = deploymentListEnabled ? viewFromQuery : "projects";
  const isDeploymentsView = currentView === "deployments";
  const projectFilter = router.query.projectId
    ? Number(router.query.projectId)
    : undefined;

  const handleViewChange = (view: string) => {
    const query: Record<string, string> = {};
    if (view !== "projects") {
      query.view = view;
    }
    void router.replace({ pathname: `/t/${team?.slug}`, query }, undefined, {
      shallow: true,
    });
  };

  return (
    <>
      <Head>{team && <title>{team.name} | Convex Dashboard</title>}</Head>
      <div className="h-full grow bg-background-primary p-4">
        <div className="m-auto max-w-3xl transition-all lg:max-w-5xl xl:max-w-7xl">
          <div className="flex w-full flex-col gap-2">
            {team && (
              <div className="w-full">
                <TeamContent
                  team={team}
                  isDeploymentsView={isDeploymentsView}
                  currentView={currentView}
                  onViewChange={handleViewChange}
                  deploymentListEnabled={deploymentListEnabled}
                  projectFilter={projectFilter}
                  referralState={referralState}
                  isFreePlan={isFreePlan}
                  prefersReferralsBannerHidden={prefersReferralsBannerHidden}
                  setPrefersReferralsBannerHidden={
                    setPrefersReferralsBannerHidden
                  }
                />
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  );
});

const VIEW_OPTIONS = [
  { label: "Projects", value: "projects" },
  { label: "Deployments", value: "deployments" },
] as const;

function TeamContent({
  team,
  isDeploymentsView,
  currentView,
  onViewChange,
  deploymentListEnabled,
  projectFilter,
  referralState,
  isFreePlan,
  prefersReferralsBannerHidden,
  setPrefersReferralsBannerHidden,
}: {
  team: TeamResponse;
  isDeploymentsView: boolean;
  currentView: string;
  onViewChange: (view: string) => void;
  deploymentListEnabled: boolean;
  projectFilter?: number;
  referralState: any;
  isFreePlan: boolean | undefined;
  prefersReferralsBannerHidden: boolean;
  setPrefersReferralsBannerHidden: (value: boolean) => void;
}) {
  const [projectQuery, setProjectQuery] = useState("");
  const [debouncedProjectQuery, setDebouncedProjectQuery] = useState("");
  const [showAsList, setShowAsList] = useGlobalLocalStorage(
    "showProjectsAsList",
    false,
  );

  useDebounce(
    () => {
      setDebouncedProjectQuery(projectQuery);
    },
    300,
    [projectQuery],
  );

  return (
    <>
      {!prefersReferralsBannerHidden && isFreePlan && referralState && (
        <div className="mb-4">
          <ReferralsBanner
            team={team}
            referralState={referralState}
            onHide={() => setPrefersReferralsBannerHidden(true)}
          />
        </div>
      )}
      <div className="mb-4 flex w-full animate-fadeInFromLoading flex-col gap-3">
        <div className="flex items-center gap-4">
          {deploymentListEnabled ? (
            <SegmentedControl
              options={[...VIEW_OPTIONS]}
              value={currentView}
              onChange={onViewChange}
            />
          ) : (
            <h3
              // eslint-disable-next-line no-restricted-syntax
              className="text-lg font-semibold"
            >
              Projects
            </h3>
          )}
          {!isDeploymentsView && <ProjectActions team={team} />}
        </div>
        {!isDeploymentsView && (
          <div className="mt-1 flex items-center gap-2">
            <div className="min-w-[13rem] shrink-0">
              <TextInput
                placeholder="Search projects"
                value={projectQuery}
                onChange={(e) => setProjectQuery(e.target.value)}
                type="search"
                id="Search projects"
                isSearchLoading={debouncedProjectQuery !== projectQuery}
              />
            </div>
            <div className="hidden gap-1 rounded-md border bg-background-secondary p-1 lg:flex">
              <Button
                icon={<GridIcon />}
                variant="neutral"
                inline
                size="xs"
                className={cn(!showAsList && "bg-background-tertiary")}
                onClick={() => setShowAsList(false)}
              />
              <Button
                icon={<ListBulletIcon />}
                variant="neutral"
                inline
                size="xs"
                className={cn(showAsList && "bg-background-tertiary")}
                onClick={() => setShowAsList(true)}
              />
            </div>
          </div>
        )}
      </div>
      {isDeploymentsView ? (
        <DeploymentsView team={team} projectFilter={projectFilter} />
      ) : (
        <ProjectGrid
          team={team}
          debouncedProjectQuery={debouncedProjectQuery}
          showAsList={showAsList}
        />
      )}
    </>
  );
}

function DeploymentsView({
  team,
  projectFilter,
}: {
  team: TeamResponse;
  projectFilter?: number;
}) {
  const filters = useDeploymentsWithFilters(team.id, projectFilter);
  return (
    <div className="flex flex-col gap-4">
      <DeploymentToolbar projectFilter={projectFilter} filters={filters} />
      <DeploymentList team={team} filters={filters} />
    </div>
  );
}

function ProjectActions({ team }: { team: TeamResponse }) {
  const [createProjectModal, showCreateProjectModal] = useCreateProjectModal();
  return (
    <>
      {!team.managedBy && (
        <Button
          onClick={() => showCreateProjectModal()}
          variant="neutral"
          size="sm"
          icon={<PlusIcon />}
          className="ml-auto"
        >
          Create Project
        </Button>
      )}
      <OpenInVercel team={team} />
      <Button
        href="https://docs.convex.dev/tutorial"
        size="sm"
        target="_blank"
        icon={<ExternalLinkIcon />}
      >
        Start Tutorial
      </Button>
      {createProjectModal}
    </>
  );
}

function ProjectGrid({
  team,
  debouncedProjectQuery,
  showAsList,
}: {
  team: TeamResponse;
  debouncedProjectQuery: string;
  showAsList: boolean;
}) {
  const { pageSize, setPageSize } = useProjectsPageSize();

  const debouncedQuery = debouncedProjectQuery;
  const [currentCursor, setCurrentCursor] = useState<string | undefined>(
    undefined,
  );
  const [cursorHistory, setCursorHistory] = useState<(string | undefined)[]>([
    undefined,
  ]);

  // Fetch paginated projects with debounced query
  const paginatedData = usePaginatedProjects(
    team?.id,
    {
      cursor: currentCursor,
      q: debouncedQuery.trim() || undefined,
    },
    30000,
  );

  const projects = paginatedData?.items ?? [];
  const hasMore = paginatedData?.pagination.hasMore ?? false;
  const nextCursor = paginatedData?.pagination.nextCursor;
  const isLoading = paginatedData === undefined;

  // Calculate current page range for display
  const currentPageNumber = cursorHistory.length;

  const handleNextPage = () => {
    if (nextCursor) {
      setCursorHistory((prev) => [...prev, nextCursor]);
      setCurrentCursor(nextCursor);
    }
  };

  const handlePrevPage = () => {
    if (cursorHistory.length > 1) {
      const newHistory = [...cursorHistory];
      newHistory.pop();
      setCursorHistory(newHistory);
      setCurrentCursor(newHistory[newHistory.length - 1]);
    }
  };

  const handlePageSizeChange = (newPageSize: number) => {
    setPageSize(newPageSize);
    // Reset to first page when page size changes
    setCurrentCursor(undefined);
    setCursorHistory([undefined]);
  };

  // Reset cursor when debounced search query changes
  useEffect(() => {
    setCurrentCursor(undefined);
    setCursorHistory([undefined]);
  }, [debouncedQuery]);

  return (
    <div className="flex flex-col items-center">
      {projects.length === 0 && isLoading && (
        <div className="my-24 flex flex-col items-center gap-2">
          <LoadingLogo />
        </div>
      )}
      {projects.length === 0 && !isLoading && debouncedQuery.trim() && (
        <div className="my-24 flex animate-fadeInFromLoading flex-col items-center gap-2 text-content-secondary">
          There are no projects matching your search.
        </div>
      )}
      {projects.length === 0 && !isLoading && !debouncedQuery.trim() && (
        <EmptySection
          header="Welcome to Convex!"
          sheet={false}
          body={
            <>
              <p className="text-sm">
                This team doesn't have any projects yet.{" "}
              </p>
              <p className="text-sm">Get started by following the tutorial.</p>
            </>
          }
          action={
            <Button
              href="https://docs.convex.dev/tutorial"
              target="_blank"
              icon={<ExternalLinkIcon />}
              className="mt-2"
            >
              Start Tutorial
            </Button>
          }
        />
      )}
      {showAsList ? (
        projects.length > 0 && (
          <div className="w-full overflow-hidden rounded-xl bg-background-secondary ring-1 ring-border-transparent">
            {projects.slice(0, pageSize).map((p: ProjectDetails, i: number) => (
              <div
                key={p.id}
                className={cn(
                  "first:rounded-t-xl last:rounded-b-xl",
                  i > 0 && "border-t",
                )}
              >
                <ProjectCard
                  project={p}
                  listItem
                  searchQuery={debouncedQuery}
                />
              </div>
            ))}
          </div>
        )
      ) : (
        <div className="grid w-full grow grid-cols-1 gap-4 lg:grid-cols-2 xl:grid-cols-3">
          {projects.slice(0, pageSize).map((p: ProjectDetails) => (
            <ProjectCard key={p.id} project={p} searchQuery={debouncedQuery} />
          ))}
        </div>
      )}

      {/* Bottom pagination controls */}
      {projects.length > 0 && (
        <div className="mt-4 mb-4 flex w-full justify-end">
          <PaginationControls
            showPageSize
            isCursorBasedPagination
            currentPage={currentPageNumber}
            hasMore={hasMore}
            pageSize={pageSize}
            onPageSizeChange={handlePageSizeChange}
            onPreviousPage={handlePrevPage}
            onNextPage={handleNextPage}
            canGoPrevious={cursorHistory.length > 1}
            pageSizeOptions={PROJECT_PAGE_SIZES}
          />
        </div>
      )}
    </div>
  );
}
