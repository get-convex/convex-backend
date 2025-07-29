import {
  useDeleteAppAccessTokenByName,
  useTeamAppAccessTokens,
} from "api/accessTokens";
import { AppAccessTokenResponse, Team } from "generatedApi";
import { AuthorizedApplications } from "components/AuthorizedApplications";
import { OauthApps } from "components/teamSettings/OauthApps";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import React from "react";
import { Tab as HeadlessTab } from "@headlessui/react";
import { Tab } from "@ui/Tab";
import { useRouter } from "next/router";

export function ApplicationsLayout({ team }: { team: Team }) {
  const router = useRouter();
  // Determine selected tab based on route
  const isOauthApps = router.pathname.endsWith("/oauth-apps");
  const selectedIndex = isOauthApps ? 1 : 0;

  const teamAccessTokens = useTeamAppAccessTokens(team.id);
  const deleteTeamAccessToken = useDeleteAppAccessTokenByName({
    teamId: team.id,
  });

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
              Manage all projects on the team
              <Tooltip tip="This includes actions like deleting projects, managing custom domains, managing project environment variable defaults, and managing cloud backups and restores.">
                <InfoCircledIcon />
              </Tooltip>
            </span>
          </li>
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
    </>
  );

  return (
    <div className="flex min-w-fit flex-col">
      <HeadlessTab.Group
        selectedIndex={selectedIndex}
        onChange={(index) => {
          const base = `/t/${team.slug}/settings/applications`;
          if (index === 0) {
            void router.push(base);
          } else {
            void router.push(`${base}/oauth-apps`);
          }
        }}
      >
        <div className="sticky top-0 z-10 bg-background-primary">
          <h2 className="mb-4">Applications</h2>
          <div className="mb-4 flex gap-2">
            <Tab>Authorized Applications</Tab>
            <Tab>Your OAuth Applications</Tab>
          </div>
        </div>
        <HeadlessTab.Panels>
          <HeadlessTab.Panel
            className="focus-visible:outline-none"
            tabIndex={-1}
          >
            <AuthorizedApplications
              accessTokens={teamAccessTokens}
              explainer={explainer}
              onRevoke={async (token: AppAccessTokenResponse) => {
                await deleteTeamAccessToken({ name: token.name });
              }}
            />
          </HeadlessTab.Panel>
          <HeadlessTab.Panel
            className="focus-visible:outline-none"
            tabIndex={-1}
          >
            <OauthApps teamId={team.id} />
          </HeadlessTab.Panel>
        </HeadlessTab.Panels>
      </HeadlessTab.Group>
    </div>
  );
}
