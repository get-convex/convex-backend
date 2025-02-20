import { useAuth0 } from "hooks/useAuth0";
import { ChevronLeftIcon } from "@radix-ui/react-icons";
import { BreadcrumbLink } from "components/header/BreadcrumbLink/BreadcrumbLink";
import { Header } from "components/header/Header/Header";
import { NavBar } from "components/header/NavBar/NavBar";
import { CreateTeamModal } from "components/header/CreateTeamModal";
import { logEvent } from "convex-analytics";
import { useTeams } from "api/teams";
import { useProjects } from "api/projects";
import {
  useRememberLastViewedProject,
  useRememberLastViewedTeam,
} from "hooks/useLastViewed";
import Link from "next/link";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { useAccessToken } from "hooks/useServerSideData";
import { ProjectSelector } from "components/header/ProjectSelector/ProjectSelector";
import { useCreateProjectModal } from "hooks/useCreateProjectModal";
import { Team } from "generatedApi";

import { PROVISION_PROD_PAGE_NAME } from "dashboard-common/lib/deploymentContext";
import { UsageBanner, useCurrentUsageBanner } from "./UsageBanner";
import {
  FailedPaymentBanner,
  useShowFailedPaymentBanner,
} from "./FailedPaymentBanner";
import {
  UpdateBillingAddressBanner,
  useShowUpdateBillingAddressBanner,
} from "./UpdateBillingAddressBanner";

export function DashboardHeader() {
  const [accessToken] = useAccessToken();

  return accessToken ? <DashboardHeaderWhenLoggedIn /> : <div />;
}

const NO_TEAM_ROUTES = [
  "/invite/[inviteId]",
  "/auth",
  "/accept",
  "/suspended",
  "/verify",
];

const NO_HEADER_ROUTES = ["/oauth/authorize/project"];

function DashboardHeaderWhenLoggedIn() {
  const { user } = useAuth0();
  const router = useRouter();

  const projectSlug = router?.query.project as string;
  const { teams, selectedTeamSlug } = useTeams();
  const team = teams?.find((t) => t.slug === selectedTeamSlug);
  const projects = useProjects(team?.id);

  const selectedProject =
    projects && projects.find((project) => project.slug === projectSlug);

  useEffect(() => {
    if (projects && projectSlug && !selectedProject) {
      void router.push("/404");
    }
  }, [projects, selectedProject, projectSlug, router]);

  const [showCreateTeamModal, setShowCreateTeamModal] = useState(false);

  useRememberLastViewedTeam(selectedTeamSlug);
  useRememberLastViewedProject(projectSlug);

  const [createProjectModal, showCreateProjectModal] = useCreateProjectModal();

  const projectSelector = (
    <ProjectSelector
      teams={teams}
      selectedProject={selectedProject}
      selectedTeamSlug={selectedTeamSlug}
      onCreateTeamClick={() => {
        logEvent("view create team modal");
        setShowCreateTeamModal(true);
      }}
      onCreateProjectClick={(t: Team) => {
        logEvent("view create project modal");
        showCreateProjectModal(t);
      }}
    />
  );
  const inNoTeamRoute = NO_TEAM_ROUTES.some((r) => r === router.pathname);
  const inNoHeaderRoute = NO_HEADER_ROUTES.some((r) => r === router.pathname);
  const getHeaderContent = () => {
    if (inNoTeamRoute) {
      return null;
    }
    if (router.route === "/profile") {
      return (
        <div className="flex items-center gap-2">
          <Link
            href="/"
            className="flex items-center gap-1 rounded px-1 py-1.5 text-xs text-content-secondary hover:bg-background-tertiary"
          >
            <ChevronLeftIcon />
            Back
          </Link>
          <BreadcrumbLink href="/profile" className="truncate">
            Profile Settings
          </BreadcrumbLink>
        </div>
      );
    }

    if (
      router.route.endsWith("/[project]/settings") ||
      router.route.endsWith("/[project]") ||
      router.route.includes("/[project]/[deploymentName]") ||
      router.route.includes(`/[project]/${PROVISION_PROD_PAGE_NAME}`)
    ) {
      return (
        <div className="flex items-center gap-4">
          {selectedProject && projectSelector}
        </div>
      );
    }

    return (
      <div className="flex items-center gap-6">
        {projectSelector}
        <NavBar
          items={[
            { label: "Projects", href: `/t/${selectedTeamSlug}` },
            {
              label: "Team Settings",
              href: `/t/${selectedTeamSlug}/settings`,
            },
          ]}
          activeLabel={
            router.asPath.startsWith(`/t/${selectedTeamSlug}/settings`)
              ? "Team Settings"
              : "Projects"
          }
        />
      </div>
    );
  };

  // Make sure that only one banner shows up at a time because of the way the
  // layout is set up right now
  const usageBannerVariant = useCurrentUsageBanner(team?.id ?? null);
  const showFailedPaymentBanner = useShowFailedPaymentBanner();
  const showUpdateBillingAddressBanner = useShowUpdateBillingAddressBanner();

  if (inNoHeaderRoute) {
    return null;
  }

  return (
    <div className="sticky top-0 z-40">
      <Header user={user}>{getHeaderContent()}</Header>

      {showFailedPaymentBanner && <FailedPaymentBanner />}
      {!showFailedPaymentBanner && showUpdateBillingAddressBanner && (
        <UpdateBillingAddressBanner />
      )}
      {!showFailedPaymentBanner &&
        !showUpdateBillingAddressBanner &&
        usageBannerVariant !== null && (
          <UsageBanner team={team!} variant={usageBannerVariant} />
        )}

      {showCreateTeamModal && (
        <CreateTeamModal onClose={() => setShowCreateTeamModal(false)} />
      )}
      {createProjectModal}
    </div>
  );
}
