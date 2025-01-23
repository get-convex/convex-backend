import { DeleteProjectModal } from "components/projects/modals/DeleteProjectModal";
import {
  PageContent,
  Loading,
  Button,
  Sheet,
  Callout,
  LocalDevCallout,
} from "dashboard-common";
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
import { useState } from "react";
import { ProjectForm } from "components/projects/ProjectForm";
import { TrashIcon } from "@radix-ui/react-icons";
import {
  LostAccessCommand,
  LostAccessDescription,
} from "components/projects/modals/LostAccessModal";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DefaultEnvironmentVariables } from "components/deploymentSettings/DefaultEnvironmentVariables";
import { DeploymentType } from "components/deploymentSettings/DeploymentUrl";
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
import { CustomDomains } from "components/deploymentSettings/CustomDomains";
import { TransferProject } from "components/projects/TransferProject";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function SettingsPage() {
  return (
    <PageContent>
      <ProjectSettings />
    </PageContent>
  );
});

function ProjectSettings() {
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const entitlements = useTeamEntitlements(team?.id);
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);
  const { projectTransfer } = useLaunchDarkly();

  return (
    <>
      <Head>
        {project && (
          <title>Project Settings | {project.name} | Convex Dashboard</title>
        )}
      </Head>
      <div className="m-auto flex max-w-[60rem] grow flex-col px-6 pb-6">
        <h2 className="sticky top-0 z-10 bg-background-primary py-6">
          Project Settings
        </h2>
        <div className="flex flex-col gap-6">
          {team && project ? (
            <ProjectForm
              team={team}
              project={project}
              hasAdminPermissions={hasAdminPermissions}
            />
          ) : (
            <Loading className="h-[50rem]" fullHeight={false} />
          )}
          <MemberProjectRoles />
          {team && project && (
            <Sheet>
              <h3 className="mb-4">Project Usage</h3>
              <p className="text-sm">
                View this project's usage and limits on{" "}
                <Link
                  className="text-content-link hover:underline dark:underline"
                  href={`/t/${team.slug}/settings/usage?projectSlug=${project.slug}`}
                >
                  this team's usage page
                </Link>
                .
              </p>
            </Sheet>
          )}
          {team && entitlements && (
            <CustomDomains
              team={team}
              hasEntitlement={entitlements.customDomainsEnabled ?? false}
            />
          )}
          {project && (
            <GenerateDeployKey
              project={project}
              hasAdminPermissions={hasAdminPermissions}
            />
          )}
          <DefaultEnvironmentVariables />
          {team && project && !project?.isDemo && (
            <LostAccess teamSlug={team.slug} projectSlug={project.slug} />
          )}
          {projectTransfer && <TransferProject />}
          <DeleteProject />
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
        <PreviewDeployKeys
          project={project}
          hasAdminPermissions={hasAdminPermissions}
        />
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
      <Link
        passHref
        href={prodHref}
        className="text-content-link dark:underline"
      >
        <DeploymentType deploymentType="prod" /> deployment
      </Link>
      . Generate and copy this key to configure Convex integrations, such as
      automatically deploying to a{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting"
        className="text-content-link dark:underline"
        target="_blank"
      >
        hosting provider
      </Link>{" "}
      (like Netlify or Vercel) or syncing data with{" "}
      <Link
        passHref
        href="https://docs.convex.dev/database/import-export/streaming"
        className="text-content-link dark:underline"
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

function PreviewDeployKeys({
  project,
  hasAdminPermissions,
}: {
  project: ProjectDetails;
  hasAdminPermissions: boolean;
}) {
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
  const disabledReason = !hasAdminPermissions ? "CannotManageProd" : null;

  const deployKeyDescription = (
    <p className="mb-2 text-sm text-content-primary">
      This key is for creating{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting/preview-deployments"
        className="text-content-link dark:underline"
        target="_blank"
      >
        preview deployments
      </Link>
      . Generate and copy this key to integrate Convex with a{" "}
      <Link
        passHref
        href="https://docs.convex.dev/production/hosting"
        className="text-content-link dark:underline"
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
            {" are only available in paid plans. "}
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
          disabledReason={disabledReason}
          buttonProps={{
            deploymentType: "preview",
            disabledReason,
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
