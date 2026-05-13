import {
  useTeamAccessTokens,
  useCreateTeamAccessToken,
} from "api/accessTokens";
import {
  useHasCustomRolePermission,
  useIsCurrentMemberTeamAdmin,
} from "api/roles";
import { useProfile } from "api/profile";
import { TeamResponse } from "generatedApi";
import { TeamAccessTokens } from "components/teamSettings/TeamAccessTokens";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
import { teamTokenResource } from "lib/permissions";
import React from "react";
import { useAccessToken } from "hooks/useServerSideData";

export function TokensLayout({ team }: { team: TeamResponse }) {
  const profile = useProfile();
  // The list endpoint only returns the current member's own tokens, so we
  // evaluate against a token resource scoped to that member (matches roles
  // like `token:creator=<self>`).
  const tokenResource = teamTokenResource(profile?.id ?? null);
  const canViewTokens = useHasCustomRolePermission(
    team.id,
    "team:token:view",
    tokenResource,
    true,
  );
  // Team token creation is admin-only by default; built-in developer
  // members cannot create team access tokens unless a custom role explicitly
  // grants `team:token:create`.
  const isTeamAdmin = useIsCurrentMemberTeamAdmin();
  const canCreateCustom = useHasCustomRolePermission(
    team.id,
    "team:token:create",
    tokenResource,
    false,
  );
  const canCreateTokens = isTeamAdmin || canCreateCustom === true;
  const canDeleteTokens = useHasCustomRolePermission(
    team.id,
    "team:token:delete",
    tokenResource,
    true,
  );
  const [accessToken] = useAccessToken();
  const teamAccessTokens = useTeamAccessTokens(team.id);
  const createTeamAccessToken = useCreateTeamAccessToken(team.id);

  if (canViewTokens === false) {
    return (
      <div className="flex min-w-fit flex-col">
        <div className="sticky top-0 z-10 bg-background-primary">
          <div className="mb-4 flex items-center justify-between">
            <h2>Team Access Tokens</h2>
          </div>
        </div>
        <NoPermissionMessage
          message="You do not have permission to view team access tokens created by you."
          missingPermission="team:token:view"
        />
      </div>
    );
  }

  const handleCreateToken = async ({
    tokenName,
    expiresAt,
  }: {
    tokenName: string;
    expiresAt?: number;
  }) => {
    await createTeamAccessToken({
      authnToken: accessToken,
      deviceName: tokenName,
      teamId: team.id,
      ...(expiresAt !== undefined && { expiresAt }),
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
        canCreate={canCreateTokens}
        canDelete={canDeleteTokens}
      />
    </div>
  );
}
