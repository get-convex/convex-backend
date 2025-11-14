import {
  ExternalLinkIcon,
  GridIcon,
  ListBulletIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { TextInput } from "@ui/TextInput";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { ProjectCard } from "components/projects/ProjectCard";
import { useProjects } from "api/projects";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import { useTeamOrbSubscription } from "api/billing";
import { useReferralState } from "api/referrals";
import { ProjectDetails, TeamResponse } from "generatedApi";
import Link from "next/link";
import { ReferralsBanner } from "components/referral/ReferralsBanner";
import { DocsGrid } from "components/projects/DocsGrid";
import { useCreateProjectModal } from "hooks/useCreateProjectModal";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import Head from "next/head";
import { useState, useEffect } from "react";
import { cn } from "@ui/cn";
import { PaginationControls } from "elements/PaginationControls";
import { usePagination } from "hooks/usePagination";
import { EmptySection } from "@common/elements/EmptySection";
import { OpenInVercel } from "components/OpenInVercel";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => {
  const team = useCurrentTeam();
  const projects = useProjects(team?.id, 30000);
  const nonDemoProjects = projects?.filter((p) => !p.isDemo);
  const entitlements = useTeamEntitlements(team?.id);
  const referralState = useReferralState(team?.id);
  const [showAsList] = useGlobalLocalStorage("showProjectsAsList", false);
  const { subscription } = useTeamOrbSubscription(team?.id);
  const isFreePlan =
    subscription === undefined ? undefined : subscription === null;
  const [prefersReferralsBannerHidden, setPrefersReferralsBannerHidden] =
    useGlobalLocalStorage("prefersReferralsBannerHidden", false);
  const isReferralsBannerVisible =
    projects &&
    projects.length > 0 &&
    isFreePlan &&
    team &&
    referralState &&
    !prefersReferralsBannerHidden;

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
            {team && nonDemoProjects && (
              <div className="w-full">
                {entitlements &&
                  nonDemoProjects.length >= entitlements.maxProjects &&
                  (subscription ? (
                    <Callout className="mb-4" variant="upsell">
                      You've reached a soft limit on the number of projects you
                      can create for this team. Please contact support to
                      increase this limit.
                    </Callout>
                  ) : (
                    <Callout className="mb-4" variant="upsell">
                      <div className="flex gap-1">
                        You've reached the project limit for this team.
                        <Link
                          href={`/${team?.slug}/settings/billing`}
                          className="items-center text-content-link"
                        >
                          Upgrade
                        </Link>
                        to create more projects.
                      </div>
                    </Callout>
                  ))}

                {isReferralsBannerVisible && (
                  <ReferralsBanner
                    team={team}
                    referralState={referralState}
                    onHide={() => setPrefersReferralsBannerHidden(true)}
                  />
                )}

                <ProjectGrid projects={nonDemoProjects} team={team} />
              </div>
            )}
          </div>
          <DocsGrid />
        </div>
      </div>
    </>
  );
});

function ProjectGrid({
  team,
  projects,
}: {
  team: TeamResponse;
  projects: ProjectDetails[];
}) {
  const [createProjectModal, showCreateProjectModal] = useCreateProjectModal();
  const [showAsList, setShowAsList] = useGlobalLocalStorage(
    "showProjectsAsList",
    false,
  );

  const [projectQuery, setProjectQuery] = useState("");

  const {
    visibleItems: paginatedProjects,
    totalPages,
    currentPage,
    setCurrentPage,
  } = usePagination({
    items: projects
      .filter((p) => p.name.toLowerCase().includes(projectQuery.toLowerCase()))
      .sort((a, b) => b.createTime - a.createTime),
    itemsPerPage: 100,
  });

  // Reset to first page when search query changes
  useEffect(() => {
    setCurrentPage(1);
  }, [projectQuery, setCurrentPage]);

  return (
    <div className="flex flex-col items-center">
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
          {paginatedProjects.length > 0 && (
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
      <PaginationControls
        currentPage={currentPage}
        totalPages={totalPages}
        onPageChange={setCurrentPage}
        className="mb-2 ml-auto"
      />
      {projects.length > 0 && paginatedProjects.length === 0 && (
        <div className="my-24 flex flex-col items-center gap-2 text-content-secondary">
          There are no projects matching your search.
        </div>
      )}
      {projects.length === 0 && (
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
        {paginatedProjects.map((p: ProjectDetails) => (
          <ProjectCard key={p.id} project={p} />
        ))}
      </div>

      <PaginationControls
        currentPage={currentPage}
        totalPages={totalPages}
        onPageChange={setCurrentPage}
        className="ml-auto"
      />

      {createProjectModal}
    </div>
  );
}
