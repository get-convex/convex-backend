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
  useProjectAppAccessTokens,
  useDeleteAppAccessTokenByName,
} from "api/accessTokens";
import { useHasProjectAdminPermissions } from "api/roles";
import { useRouter } from "next/router";
import { useState, useEffect } from "react";
import { ProjectForm } from "components/projects/ProjectForm";
import { TrashIcon, InfoCircledIcon } from "@radix-ui/react-icons";
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
import { AuthorizedApplications } from "components/AuthorizedApplications";
import { Tooltip } from "@ui/Tooltip";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function ProjectSettingsPage() {
  return (
    <PageContent>
      <ProjectSettings />
    </PageContent>
  );
});

const SECTION_IDS = {
  projectForm: "project-form",
  projectRoles: "project-roles",
  projectUsage: "project-usage",
  customDomains: "custom-domains",
  deployKeys: "deploy-keys",
  authorizedApps: "applications",
  envVars: "env-vars",
  lostAccess: "lost-access",
  transferProject: "transfer-project",
  deleteProject: "delete-project",
} as const;

const sections = [
  { id: SECTION_IDS.projectForm, label: "Edit Project" },
  { id: SECTION_IDS.projectRoles, label: "Project Admins" },
  { id: SECTION_IDS.projectUsage, label: "Project Usage" },
  { id: SECTION_IDS.customDomains, label: "Custom Domains" },
  { id: SECTION_IDS.deployKeys, label: "Deploy Keys" },
  {
    id: SECTION_IDS.authorizedApps,
    label: "Authorized Applications",
  },
  { id: SECTION_IDS.envVars, label: "Environment Variables" },
  { id: SECTION_IDS.lostAccess, label: "Lost Access" },
  { id: SECTION_IDS.transferProject, label: "Transfer Project" },
  { id: SECTION_IDS.deleteProject, label: "Delete Project" },
];

function SettingsNavigation() {
  return (
    <nav
      data-settings-nav
      className="relative"
      aria-label="Settings navigation"
    >
      <div
        className="absolute left-0 h-full w-0.5 rounded-sm bg-background-tertiary"
        aria-hidden="true"
      />
      <SettingsNavigationScrollProgress />
      <ul className="pl-1 text-sm">
        {sections.map(({ id, label }) => (
          <li key={id} className="py-px">
            <a
              href={`#${id}`}
              className={cn(
                "block rounded-sm px-2 py-2 transition-all duration-200",
                "text-content-primary hover:bg-background-secondary",
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

                  window.history.pushState(null, "", `#${id}`);
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

function SettingsNavigationScrollProgress() {
  const [transform, setTransform] = useState<string | undefined>(undefined);

  useEffect(() => {
    const contentContainer = document.querySelector("[data-settings-content]");
    if (!contentContainer) return undefined;

    const forceUpdate = () => {
      const containerRect = contentContainer.getBoundingClientRect();

      const elementHeight = 1 / sections.length;

      const firstBoundary = findScrollBoundary("first", containerRect);
      const lastBoundary = findScrollBoundary("last", containerRect);

      const y =
        (firstBoundary.index + (1 - firstBoundary.visibilityFraction)) *
        elementHeight;
      const height =
        firstBoundary.index === lastBoundary.index
          ? firstBoundary.visibilityFraction * elementHeight
          : (firstBoundary.visibilityFraction +
              lastBoundary.visibilityFraction +
              lastBoundary.index -
              firstBoundary.index -
              1) *
            elementHeight;

      setTransform(`translateY(${y * 100}%) scaleY(${height})`);
    };

    forceUpdate(); // Initial calculation

    const update = () => {
      window.requestAnimationFrame(forceUpdate);
    };
    contentContainer.addEventListener("scroll", update);
    window.addEventListener("resize", update);
    return () => {
      contentContainer.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, []);

  if (transform === undefined) return null;

  return (
    <div
      className="absolute left-0 h-full w-0.5 origin-top rounded-sm bg-content-primary"
      style={{
        transform,
      }}
      aria-hidden="true"
    />
  );
}

function findScrollBoundary(
  boundary: "first" | "last",
  containerRect: DOMRect,
) {
  for (
    let i = boundary === "first" ? 0 : sections.length - 1;
    boundary === "first" ? i < sections.length : i >= 0;
    boundary === "first" ? i++ : i--
  ) {
    const section = sections[i];
    const element = document.getElementById(section.id);
    if (!element) {
      continue;
    }

    const rect = element.getBoundingClientRect();

    const visibleHeight =
      Math.min(rect.bottom, containerRect.bottom) -
      Math.max(rect.top, containerRect.top);

    if (visibleHeight > 0) {
      const elementHeight = rect.height;
      return {
        index: i,
        visibilityFraction: visibleHeight / elementHeight,
      };
    }
  }

  return {
    index: 0,
    visibilityFraction: 0,
  };
}

function ProjectSettings() {
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const entitlements = useTeamEntitlements(team?.id);
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);
  const router = useRouter();

  const projectAppAccessTokens = useProjectAppAccessTokens(project?.id);
  const deleteAppAccessTokenByName = useDeleteAppAccessTokenByName({
    projectId: project?.id!,
  });

  const { showTeamOauthTokens } = useLaunchDarkly();

  const authorizedAppsExplainer = (
    <>
      <h3 className="mb-2">Authorized Applications</h3>
      <p className="text-sm text-content-primary">
        These 3rd-party applications have been authorized to access this project
        on your behalf.
      </p>
      <div className="mt-2 mb-2 text-sm text-content-primary">
        <span className="font-semibold">
          What can authorized applications do?
        </span>
        <ul className="mt-1 list-disc pl-4">
          <li>Create new deployments in this project</li>
          <li>
            <span className="flex items-center gap-1">
              Read and write data in any deployment in this project
              <Tooltip tip="Write access to Production deployments will depend on your team-level and project-level roles.">
                <InfoCircledIcon />
              </Tooltip>
            </span>
          </li>
        </ul>
      </div>
      <p className="mt-1 mb-2 text-sm text-content-primary">
        You cannot see applications that other members of your team have
        authorized.
      </p>
      {team && showTeamOauthTokens && (
        <p className="mt-1 mb-2 text-xs text-content-secondary">
          There may also be <b>team-wide authorized applications</b> that can
          access all projects in this team. You can view them in{" "}
          <Link
            href={`/t/${team.slug}/settings/applications`}
            className="text-content-link hover:underline"
          >
            Team Settings
          </Link>
          .
        </p>
      )}
    </>
  );

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

  const title = <h2 className="pointer-events-auto py-6">Project Settings</h2>;

  return (
    <>
      <Head>
        {project && (
          <title>Project Settings | {project.name} | Convex Dashboard</title>
        )}
      </Head>
      <div className="relative h-full [--container-px:--spacing(6)] [--container-width:80rem] [--sidebar-gap:--spacing(8)] [--sidebar-width:12rem]">
        <div className="pointer-events-none absolute inset-0 top-0 z-10 hidden md:block">
          <div className="mx-auto flex h-full max-w-(--container-width) gap-(--sidebar-gap) px-(--container-px)">
            <div className="h-full w-(--sidebar-width)">
              <div className="grid h-full grid-rows-[auto_1fr]">
                {title}
                <div className="scrollbar overflow-y-auto">
                  <div className="pointer-events-auto pb-8">
                    <SettingsNavigation />
                  </div>
                </div>
              </div>
            </div>
            <div className="grow" />
          </div>
        </div>
        <div className="scrollbar h-full overflow-y-auto" data-settings-content>
          <div className="m-auto flex min-h-0 max-w-(--container-width) gap-(--sidebar-gap) px-(--container-px)">
            <div className="hidden w-(--sidebar-width) shrink-0 md:block" />

            <div className="flex flex-col items-start">
              <div className="md:hidden">{title}</div>

              <div className="flex grow flex-col gap-6 pr-2 pb-6 md:pt-20 [&>*]:scroll-mt-3">
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
                      hasEntitlement={
                        entitlements.customDomainsEnabled ?? false
                      }
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
                  <div id={SECTION_IDS.authorizedApps}>
                    <AuthorizedApplications
                      accessTokens={projectAppAccessTokens}
                      explainer={authorizedAppsExplainer}
                      onRevoke={async (token) => {
                        await deleteAppAccessTokenByName({ name: token.name });
                      }}
                    />
                  </div>
                )}
                <div id={SECTION_IDS.envVars}>
                  <DefaultEnvironmentVariables />
                </div>
                {team && project && !project?.isDemo && (
                  <div id={SECTION_IDS.lostAccess}>
                    <LostAccess
                      teamSlug={team.slug}
                      projectSlug={project.slug}
                    />
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
        <div className="mb-4">
          <h4 className="mb-2">Production</h4>
          <p className="text-sm text-content-primary">
            This project does not have a Production deployment yet.
          </p>
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
