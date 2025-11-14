import {
  useTeamAccessTokens,
  useCreateTeamAccessToken,
} from "api/accessTokens";
import { TeamResponse } from "generatedApi";
import { TeamAccessTokens } from "components/teamSettings/TeamAccessTokens";
import React from "react";
import { useAccessToken } from "hooks/useServerSideData";

export function TokensLayout({ team }: { team: TeamResponse }) {
  const [accessToken] = useAccessToken();
  const teamAccessTokens = useTeamAccessTokens(team.id);
  const createTeamAccessToken = useCreateTeamAccessToken({
    kind: "team",
    teamId: team.id,
  });

  const handleCreateToken = async (tokenName: string) => {
    await createTeamAccessToken({
      authnToken: accessToken,
      deviceName: tokenName,
      teamId: team.id,
    });
  };

  return (
    <div className="flex min-w-fit flex-col">
      <div className="sticky top-0 z-10 bg-background-primary">
        <div className="mb-4 flex items-center justify-between">
          <h2>Team Access Tokens</h2>
        </div>
      </div>
      <TeamAccessTokens
        accessTokens={teamAccessTokens}
        onCreateToken={handleCreateToken}
      />
    </div>
  );
}
