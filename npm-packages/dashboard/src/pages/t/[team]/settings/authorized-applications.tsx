import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import {
  useTeamAppAccessTokens,
  useDeleteTeamAccessToken,
} from "api/accessTokens";
import { useCurrentTeam } from "api/teams";
import { AppAccessTokenResponse, Team } from "generatedApi";
import { AuthorizedApplications } from "components/projectSettings/AuthorizedApplications";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import React from "react";

function TeamAuthorizedApplicationsPage({ team }: { team: Team }) {
  const teamAccessTokens = useTeamAppAccessTokens(team.id);
  const deleteTeamAccessToken = useDeleteTeamAccessToken(team.id);

  const explainer = (
    <>
      <p className="text-sm text-content-primary">
        These 3rd-party applications have been authorized to access this team on
        your behalf.
      </p>
      <div className="mt-2 mb-2 text-sm text-content-primary">
        <span className="font-semibold">
          What can authorized applications do?
        </span>
        <ul className="mt-1 list-disc pl-4">
          <li>Create new projects</li>
          <li>Create new deployments</li>
          <li>
            <span className="flex items-center gap-1">
              Read and write data in all projects
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
      <p className="mt-1 mb-2 text-xs text-content-secondary">
        You can view authorized applications for each project in the respective
        Settings page for each project.
      </p>
    </>
  );

  return (
    <AuthorizedApplications
      accessTokens={teamAccessTokens}
      explainer={explainer}
      onRevoke={async (token: AppAccessTokenResponse) => {
        await deleteTeamAccessToken({ name: token.name } as any);
      }}
    />
  );
}

export default withAuthenticatedPage(() => {
  const team = useCurrentTeam();
  if (!team) return null;
  return (
    <TeamSettingsLayout
      page="authorized-applications"
      Component={() => <TeamAuthorizedApplicationsPage team={team} />}
      title="Authorized Applications"
    />
  );
});
