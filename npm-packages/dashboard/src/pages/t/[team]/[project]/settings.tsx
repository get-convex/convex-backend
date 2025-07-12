import { DeleteProjectModal } from "components/projects/modals/DeleteProjectModal";
import { PageContent } from "@common/elements/PageContent";
import { Loading } from "@ui/Loading";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { LocalDevCallout } from "@common/elements/LocalDevCallout";
import { Callout } from "@ui/Callout";
import { DeploymentType } from "@common/features/settings/components/DeploymentUrl";
import { useDeployments } from "api/deployments";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import { useCurrentProject } from "api/projects";
import {
  useCreateTeamAccessToken,
  useInstanceAccessTokens,
  useProjectAccessTokens,
} from "api/accessTokens";
import { useHasProjectAdminPermissions } from "api/roles";
import { useRouter } from "next/router";
import { useState, useEffect, useMemo } from "react";
import { ProjectForm } from "components/projects/ProjectForm";
import { TrashIcon } from "@radix-ui/react-icons";
import {
  LostAccessCommand,
  LostAccessDescription,
} from "components/projects/modals/LostAccessModal";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DefaultEnvironmentVariables } from "components/projectSettings/DefaultEnvironmentVariables";
import {
  getAccessTokenBasedDeployKey,
  getAccessTokenBasedDeployKeyForPreview,
} from "components/deploymentSettings/DeployKeysForDeployment";
import { ProjectDetails } from "generatedApi";
import Link from "next/link";
import { useDeploymentUris } from "hooks/useDeploymentUris";
import Head from "next/head";
import { useAccessToken } from "hooks/useServerSideData";
import { MemberProjectRoles } from "components/projects/MemberProjectRoles";
import { DeploymentAccessTokenList } from "components/deploymentSettings/DeploymentAccessTokenList";
import { CustomDomains } from "components/projectSettings/CustomDomains";
import { TransferProject } from "components/projects/TransferProject";
import { cn } from "@ui/cn";
import { AuthorizedApplications } from "components/projectSettings/AuthorizedApplications";

const SECTION_IDS = {
  projectForm: "project-form",
  projectRoles: "project-roles",
  projectUsage: "project-usage",
  customDomains: "custom-domains",
  deployKeys: "deploy-keys",
  authorizedApplications: "authorized-applications",
  envVars: "env-vars",
  lostAccess: "lost-access",
  transferProject: "transfer-project",
  deleteProject: "delete-project",
} as const;

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function SettingsPage() {
  return (
    <PageContent>
      <ProjectSettings />
    </PageContent>
  );
});

function SettingsNavigation() {
  const sections = useMemo(
    () => [
      { id: SECTION_IDS.projectForm, label: "Edit Project" },
      { id: SECTION_IDS.projectRoles, label: "Project Admins" },
      { id: SECTION_IDS.projectUsage, label: "Project Usage" },
      { id: SECTION_IDS.customDomains, label: "Custom Domains" },
      { id: SECTION_IDS.deployKeys, label: "Deploy Keys" },
      {
        id: SECTION_IDS.authorizedApplications,
        label: "Authorized Applications",
      },
      { id: SECTION_IDS.envVars, label: "Environment Variables" },
      { id: SECTION_IDS.lostAccess, label: "Lost Access" },
      { id: SECTION_IDS.transferProject, label: "Transfer Project" },
      { id: SECTION_IDS.deleteProject, label: "Delete Project" },
    ],
    [],
  );

  const [scrollPercentage, setScrollPercentage] = useState(0);
  const [visibleSections, setVisibleSections] = useState<Set<string>>(
    new Set(),
  );

  useEffect(() => {
    const handleScroll = () => {
      const contentContainer = document.querySelector(
        "[data-settings-content]",
      );
      if (!contentContainer) return;

      const { scrollTop, scrollHeight, clientHeight } = contentContainer;
      const percentage = (scrollTop / (scrollHeight - clientHeight)) * 100;
      setScrollPercentage(Math.min(100, Math.max(0, percentage)));

      // Check which sections are visible
      const visibleIds = new Set<string>();
      sections.forEach(({ id }) => {
        const element = document.getElementById(id);
        if (!element) return;

        const rect = element.getBoundingClientRect();
        const containerRect = contentContainer.getBoundingClientRect();

        // Calculate visibility percentage
        const elementHeight = rect.height;
        const visibleHeight =
          Math.min(rect.bottom, containerRect.bottom) -
          Math.max(rect.top, containerRect.top);
        const visibilityPercentage = (visibleHeight / elementHeight) * 100;

        // If at least 10% of the element is visible
        if (visibilityPercentage >= 10) {
          visibleIds.add(id);
        }
      });

      setVisibleSections(visibleIds);
    };

    const contentContainer = document.querySelector("[data-settings-content]");
    if (contentContainer) {
      contentContainer.addEventListener("scroll", handleScroll);
      handleScroll(); // Initial calculation
      return () => contentContainer.removeEventListener("scroll", handleScroll);
    }
  }, [sections]);

  return (
    <nav
      data-settings-nav
      className="sticky top-0 hidden h-fit w-[14rem] shrink-0 overflow-visible pr-8 md:block"
      aria-label="Settings navigation"
    >
      <div
        className="absolute left-0 h-full w-0.5 rounded-sm bg-background-tertiary"
        aria-hidden="true"
      />
      <div
        className="absolute left-0 h-8 w-0.5 rounded-sm bg-content-primary transition-transform duration-75"
        style={{
          top: `max(0%, calc(${scrollPercentage}% - 2rem))`,
        }}
        aria-hidden="true"
      />
      <ul className="space-y-1 pl-1 text-sm">
        {sections.map(({ id, label }) => (
          <li key={id}>
            <a
              href={`#${id}`}
              className={cn(
                "block rounded-sm px-2 py-1.5 transition-all duration-200",
                "text-content-secondary hover:bg-background-secondary hover:text-content-primary",
                visibleSections.has(id) && "text-content-primary",
              )}
              onClick={(e) => {
                e.preventDefault();
                const element = document.getElementById(id);
                if (element) {
                  const rect = element.getBoundingClientRect();
                  const isInView =
                    rect.top >= 0 && rect.bottom <= window.innerHeight;
                  element.scrollIntoView({
                    behavior: "smooth",
                    block: isInView ? "start" : "nearest",
                    inline: "nearest",
                  });
                }
              }}
            >
              {label}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}

function ProjectSettings() {
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const entitlements = useTeamEntitlements(team?.id);
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);
  const router = useRouter();

  useEffect(() => {
    // Handle initial scroll based on hash
    if (typeof window !== "undefined" && window.location.hash) {
      const id = window.location.hash.slice(1); // Remove the # from the hash
      const element = document.getElementById(id);
      if (element) {
        // Add a small delay to ensure the content is rendered
        setTimeout(() => {
          element.scrollIntoView({
            behavior: "smooth",
            block: "start",
            inline: "start",
          });
        }, 100);
      }
    }
  }, [team, project, router]); // Only run when team/project load since that's when content becomes available

  return (
    <>
      <Head>
        {project && (
          <title>Project Settings | {project.name} | Convex Dashboard</title>
        )}
      </Head>
      <div className="m-auto flex h-full max-w-[80rem] grow flex-col gap-6 px-6">
        <h2 className="sticky top-0 z-10 bg-background-primary pt-6">
          Project Settings
        </h2>
        <div className="flex grow flex-col items-start gap-6 overflow-y-hidden md:flex-row">
          <SettingsNavigation />
          <div
            data-settings-content
            className="scrollbar flex h-full grow flex-col gap-6 overflow-y-auto pr-2 pb-6"
          >
            {team && project ? (
              <div id={SECTION_IDS.projectForm}>
                <ProjectForm
                  team={team}
                  project={project}
                  hasAdminPermissions={hasAdminPermissions}
                />
              </div>
            ) : (
              <Loading className="h-[50rem]" fullHeight={false} />
            )}
            <div id={SECTION_IDS.projectRoles}>
              <MemberProjectRoles />
            </div>
            {team && project && (
              <Sheet id={SECTION_IDS.projectUsage}>
                <h3 className="mb-4">Project Usage</h3>
                <p className="text-sm">
                  View this project's usage and limits on{" "}
                  <Link
                    className="text-content-link hover:underline"
                    href={`/t/${team.slug}/settings/usage?projectSlug=${project.slug}`}
                  >
                    this team's usage page
                  </Link>
                  .
                </p>
              </Sheet>
            )}
            {team && entitlements && (
              <div id={SECTION_IDS.customDomains}>
                <CustomDomains
                  team={team}
                  hasEntitlement={entitlements.customDomainsEnabled ?? false}
                />
              </div>
            )}
            {project && (
              <div id={SECTION_IDS.deployKeys}>
                <GenerateDeployKey
                  project={project}
                  hasAdminPermissions={hasAdminPermissions}
                />
              </div>
            )}
            {project && (
              <div id={SECTION_IDS.authorizedApplications}>
                <AuthorizedApplications project={project} />
              </div>
            )}
            <div id={SECTION_IDS.envVars}>
              <DefaultEnvironmentVariables />
            </div>
            {team && project && !project?.isDemo && (
              <div id={SECTION_IDS.lostAccess}>
                <LostAccess teamSlug={team.slug} projectSlug={project.slug} />
              </div>
            )}
            <div id={SECTION_IDS.transferProject}>
              <TransferProject />
            </div>
            <div id={SECTION_IDS.deleteProject}>
              <DeleteProject />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

function LostAccess({
  teamSlug,
  projectSlug,
}: {
  teamSlug: string;
  projectSlug: string;
}) {
  return (
    <Sheet>
      <h3 className="mb-4">Lost Access</h3>
      <LostAccessDescription />
      <LostAccessCommand teamSlug={teamSlug} projectSlug={projectSlug} />
    </Sheet>
  );
}

function DeleteProject() {
  const router = useRouter();

  const team = useCurrentTeam();
  const project = useCurrentProject();

  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);

  const [showDeleteModal, setShowDeleteModal] = useState(false);
  return (
    <>
      <Sheet>
        <h3 className="mb-4">Delete Project</h3>
        <p className="mb-5 text-sm text-content-primary">
          Permanently delete this project for you and all team members. This
          action cannot be undone.
        </p>
        <Button
          variant="danger"
          onClick={() => setShowDeleteModal(!showDeleteModal)}
          icon={<TrashIcon />}
          disabled={!hasAdminPermissions}
          tip={
            !hasAdminPermissions
              ? "You do not have permission to delete this project."
              : undefined
          }
        >
          Delete
        </Button>
      </Sheet>
      {team && project && showDeleteModal && (
        <DeleteProjectModal
          team={team}
          project={project}
          onClose={() => setShowDeleteModal(false)}
          onDelete={async () => {
            await router.push("/");
          }}
        />
      )}
    </>
  );
}

function GenerateDeployKey({
  project,
  hasAdminPermissions,
}: {
  project: ProjectDetails;
  hasAdminPermissions: boolean;
}) {
  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Deploy Keys</h3>
      <div className="flex flex-col gap-4 divide-y">
        <ProductionDeployKeys
          project={project}
          hasAdminPermissions={hasAdminPermissions}
        />
        <PreviewDeployKeys project={project} />
      </div>
    </Sheet>
  );
}

function ProductionDeployKeys({
  project,
  hasAdminPermissions,
}: {
  project: ProjectDetails;
  hasAdminPermissions: boolean;
}) {
  const team = useCurrentTeam();
  const [accessToken] = useAccessToken();

  const { deployments } = useDeployments(project.id);
  const prodDeployment = deployments?.find((d) => d.deploymentType === "prod");
  const { prodHref } = useDeploymentUris(project.id, project.slug);

  const disabledReason = !hasAdminPermissions ? "CannotManageProd" : null;

  const accessTokens = useInstanceAccessTokens(
    disabledReason === null ? prodDeployment?.name : undefined,
  );
  const createAccessTokenMutation = useCreateTeamAccessToken({
    deploymentName: prodDeployment?.name || "",
    kind: "deployment",
  });

  const deployKeyDescription = (
    <p className="mb-2 text-sm text-content-primary">
      This is the key for your{" "}
      <Link passHref href={prodHref} className="text-content-link">
        <DeploymentType deploymentType="prod" /> deployment
      </Link>
      . Generate and copy this key to configure Convex integrations, such as
      automatically deploying to a{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting"
        className="text-content-link"
        target="_blank"
      >
        hosting provider
      </Link>{" "}
      (like Netlify or Vercel) or syncing data with{" "}
      <Link
        passHref
        href="https://docs.convex.dev/database/import-export/streaming"
        className="text-content-link"
        target="_blank"
      >
        Fivetran or Airbyte
      </Link>
      .
    </p>
  );

  return (
    <div className="flex flex-col gap-2">
      {team && prodDeployment ? (
        <DeploymentAccessTokenList
          header="Production"
          description={deployKeyDescription}
          disabledReason={disabledReason}
          buttonProps={{
            deploymentType: "prod",
            getAdminKey: async (name: string) =>
              getAccessTokenBasedDeployKey(
                prodDeployment,
                project,
                team,
                `prod:${prodDeployment.name}`,
                accessToken,
                createAccessTokenMutation,
                name,
              ),
            disabledReason,
          }}
          identifier={prodDeployment.name}
          tokenPrefix={`prod:${prodDeployment.name}`}
          accessTokens={accessTokens}
          kind="deployment"
        />
      ) : (
        <div>
          <h4 className="mb-2">Production</h4>
          This project does not have a production deployment yet.
        </div>
      )}
    </div>
  );
}

function PreviewDeployKeys({ project }: { project: ProjectDetails }) {
  const createProjectAccessTokenMutation = useCreateTeamAccessToken({
    projectId: project.id,
    kind: "project",
  });
  const [accessToken] = useAccessToken();
  const team = useCurrentTeam();
  const selectedTeamSlug = team?.slug;

  const arePreviewDeploymentsAvailable =
    useTeamEntitlements(team?.id)?.projectMaxPreviewDeployments !== 0;
  const projectAccessTokens = useProjectAccessTokens(project.id);

  const deployKeyDescription = (
    <p className="mb-2 text-sm text-content-primary">
      These keys are for creating{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting/preview-deployments"
        className="text-content-link"
        target="_blank"
      >
        preview deployments
      </Link>
      . Generate and copy a preview key to integrate Convex with a{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting"
        className="text-content-link"
        target="_blank"
      >
        hosting provider
      </Link>{" "}
      (like Netlify or Vercel) in order to view both frontend and backend
      changes before they're deployed to production.
    </p>
  );

  if (!arePreviewDeploymentsAvailable) {
    return (
      <div className="pt-4">
        <Callout>
          <p>
            <Link
              passHref
              href="https://docs.convex.dev/production/hosting/preview-deployments"
              className="underline"
              target="_blank"
            >
              Preview deployments
            </Link>
            {" are only available on the Pro plan. "}
            <Link
              href={`/${selectedTeamSlug}/settings/billing`}
              className="underline"
            >
              Upgrade to get access.
            </Link>
          </p>
        </Callout>
        <LocalDevCallout
          tipText="Tip: Run this to enable preview deployments locally:"
          command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement project_max_preview_deployments --team-id ${team?.id} --reason "local" 200 --for-real`}
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 pt-4">
      {team && accessToken && createProjectAccessTokenMutation && (
        <DeploymentAccessTokenList
          identifier={project.id.toString()}
          tokenPrefix={`preview:${selectedTeamSlug}:${project.slug}`}
          accessTokens={projectAccessTokens}
          kind="project"
          disabledReason={null}
          buttonProps={{
            deploymentType: "preview",
            disabledReason: null,
            getAdminKey: async (name: string) =>
              getAccessTokenBasedDeployKeyForPreview(
                project,
                team,
                `preview:${selectedTeamSlug}:${project.slug}`,
                accessToken,
                createProjectAccessTokenMutation,
                name,
              ),
          }}
          header="Preview"
          description={deployKeyDescription}
        />
      )}
    </div>
  );
}
