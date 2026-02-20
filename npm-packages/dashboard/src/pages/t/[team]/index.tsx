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
import { EmptySection } from "@common/elements/EmptySection";
import { OpenInVercel } from "components/OpenInVercel";
import { LoadingLogo } from "@ui/Loading";
import { PaginationControls } from "elements/PaginationControls";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => {
  const team = useCurrentTeam();
  const referralState = useReferralState(team?.id);
  const [showAsList] = useGlobalLocalStorage("showProjectsAsList", false);
  const { subscription } = useTeamOrbSubscription(team?.id);
  const isFreePlan =
    subscription === undefined ? undefined : subscription === null;
  const [prefersReferralsBannerHidden, setPrefersReferralsBannerHidden] =
    useGlobalLocalStorage("prefersReferralsBannerHidden", false);

  return (
    <>
      <Head>{team && <title>{team.name} | Convex Dashboard</title>}</Head>
      <div className="h-full grow bg-background-primary p-4">
        <div
          className={cn(
            "m-auto transition-all",
            showAsList ? "max-w-3xl" : "max-w-3xl lg:max-w-5xl xl:max-w-7xl",
          )}
        >
          <div className="flex w-full flex-col gap-2">
            {team && (
              <div className="w-full">
                <ProjectGrid
                  team={team}
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

function ProjectGrid({
  team,
  referralState,
  isFreePlan,
  prefersReferralsBannerHidden,
  setPrefersReferralsBannerHidden,
}: {
  team: TeamResponse;
  referralState: any;
  isFreePlan: boolean | undefined;
  prefersReferralsBannerHidden: boolean;
  setPrefersReferralsBannerHidden: (value: boolean) => void;
}) {
  const [createProjectModal, showCreateProjectModal] = useCreateProjectModal();
  const [showAsList, setShowAsList] = useGlobalLocalStorage(
    "showProjectsAsList",
    false,
  );
  const { pageSize, setPageSize } = useProjectsPageSize();

  const [projectQuery, setProjectQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [currentCursor, setCurrentCursor] = useState<string | undefined>(
    undefined,
  );
  const [cursorHistory, setCursorHistory] = useState<(string | undefined)[]>([
    undefined,
  ]);

  // Debounce search query (300ms delay)
  useDebounce(
    () => {
      setDebouncedQuery(projectQuery);
    },
    300,
    [projectQuery],
  );

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

  const isReferralsBannerVisible =
    projects.length > 0 &&
    isFreePlan &&
    referralState &&
    !prefersReferralsBannerHidden;

  const handleNextPage = () => {
    if (nextCursor) {
      setCursorHistory((prev) => [...prev, currentCursor]);
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
      {isReferralsBannerVisible && (
        <ReferralsBanner
          team={team}
          referralState={referralState}
          onHide={() => setPrefersReferralsBannerHidden(true)}
        />
      )}

      <div className="mb-4 flex w-full animate-fadeInFromLoading flex-col flex-wrap gap-4 sm:flex-row sm:items-center">
        <h3>Projects</h3>
        <div className="flex flex-wrap gap-2 sm:ml-auto sm:flex-nowrap">
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
          <TextInput
            outerClassname="min-w-[13rem] max-w-xs"
            placeholder="Search projects"
            value={projectQuery}
            onChange={(e) => setProjectQuery(e.target.value)}
            type="search"
            id="Search projects"
            isSearchLoading={isLoading && debouncedQuery === projectQuery}
          />
          {!team.managedBy && (
            <Button
              onClick={() => showCreateProjectModal()}
              variant="neutral"
              size="sm"
              icon={<PlusIcon />}
            >
              Create Project
            </Button>
          )}
          <OpenInVercel team={team} />
          {projects.length > 0 && (
            <Button
              href="https://docs.convex.dev/tutorial"
              size="sm"
              target="_blank"
              icon={<ExternalLinkIcon />}
            >
              Start Tutorial
            </Button>
          )}
        </div>
      </div>

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
      <div
        className={cn(
          "mb-4 grid w-full grow grid-cols-1 gap-4",
          !showAsList && "lg:grid-cols-2 xl:grid-cols-3",
        )}
      >
        {/* In case the page returned more items than requested, slice the result down to the page size. This only happens for the first page of SSRed data. */}
        {projects.slice(0, pageSize).map((p: ProjectDetails) => (
          <ProjectCard key={p.id} project={p} />
        ))}
      </div>

      {/* Bottom pagination controls */}
      {projects.length > 0 && (
        <div className="mb-4 flex w-full justify-end">
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

      {createProjectModal}
    </div>
  );
}
