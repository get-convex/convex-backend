import { TeamAccessTokenResponse } from "generatedApi";
import { Sheet } from "@ui/Sheet";
import { LoadingTransition } from "@ui/Loading";
import { AccessTokenListItem } from "components/AccessTokenListItem";
import { Button } from "@ui/Button";
import { InfoCircledIcon, PlusIcon } from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import React, { useState } from "react";
import { CreateTokenDialog } from "components/teamSettings/CreateTokenDialog";
import { useCurrentTeam } from "api/teams";
import Link from "next/link";

export function TeamAccessTokens({
  accessTokens,
  onCreateToken,
}: {
  accessTokens: TeamAccessTokenResponse[] | undefined;
  onCreateToken: (tokenName: string) => Promise<void>;
}) {
  const team = useCurrentTeam();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  return (
    <Sheet>
      <div className="mb-4 flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <p className="text-sm text-content-primary">
            These access tokens allow your team to access your Convex projects
            using{" "}
            <Link
              href="https://docs.convex.dev/platform-apis"
              className="text-content-link hover:underline"
              target="_blank"
            >
              Convex Platform APIs
            </Link>
            .
          </p>
          <Button onClick={() => setShowCreateDialog(true)} icon={<PlusIcon />}>
            Create Token
          </Button>
        </div>
        <div>
          <div className="flex items-center gap-1">
            <span className="font-semibold">Team ID</span>
            <Tooltip
              side="right"
              tip="This is the ID of your team. It is used to identify your team in the Platform API."
            >
              <InfoCircledIcon />
            </Tooltip>
          </div>
          <span className="font-mono">{team?.id}</span>
        </div>
        <div className="mt-2 mb-2 text-sm text-content-primary">
          <span className="font-semibold">What can team access tokens do?</span>
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
          You cannot see tokens that other members of your team have created.
        </p>
      </div>
      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "h-14 w-full" }}
      >
        {accessTokens !== undefined && (
          <div className="mt-2 flex w-full flex-col gap-2 divide-y">
            {team && accessTokens.length > 0 ? (
              [...accessTokens]
                .sort((a, b) => b.creationTime - a.creationTime)
                .map((token) => (
                  <AccessTokenListItem
                    kind="team"
                    key={token.name}
                    token={token}
                    identifier={team.id.toString()}
                    shouldShow={false}
                    showMemberName={false}
                    showCallout={false}
                  />
                ))
            ) : (
              <div className="my-6 flex w-full justify-center text-content-secondary">
                You have not created any team access tokens yet.
              </div>
            )}
          </div>
        )}
      </LoadingTransition>
      {showCreateDialog && (
        <CreateTokenDialog
          onClose={() => setShowCreateDialog(false)}
          onSubmit={async (tokenName: string) => {
            await onCreateToken(tokenName);
            setShowCreateDialog(false);
          }}
        />
      )}
    </Sheet>
  );
}
