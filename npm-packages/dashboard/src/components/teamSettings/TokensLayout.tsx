import {
  useTeamAccessTokens,
  useCreateTeamAccessToken,
} from "api/teamAccessTokens";
import { useHasCustomRolePermission } from "api/roles";
import { useProfile } from "api/profile";
import { TeamResponse } from "generatedApi";
import { TeamAccessTokens } from "components/teamSettings/TeamAccessTokens";
import { NoPermissionMessage } from "elements/NoPermissionMessage";
import { teamTokenResource } from "lib/permissions";
import React, { useState } from "react";

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
  // Both built-in admin and developer members can create their own team
  // access tokens; custom-role members need `team:token:create`. A team
  // access token can only perform actions its creator is allowed to perform,
  // so creating one never escalates the creator's privileges.
  const canCreateTokens = useHasCustomRolePermission(
    team.id,
    "team:token:create",
    tokenResource,
    true,
  );
  const canDeleteTokens = useHasCustomRolePermission(
    team.id,
    "team:token:delete",
    tokenResource,
    true,
  );
  const [currentCursor, setCurrentCursor] = useState<string | undefined>(
    undefined,
  );
  const [cursorHistory, setCursorHistory] = useState<(string | undefined)[]>([
    undefined,
  ]);

  const { data, isLoading } = useTeamAccessTokens(team.id, currentCursor);

  const tokens = data?.items;
  const hasMore = data?.pagination.hasMore ?? false;
  const nextCursor = data?.pagination.nextCursor;
  const currentPage = cursorHistory.length;

  const handleNextPage = () => {
    if (nextCursor) {
      setCursorHistory((prev) => [...prev, nextCursor]);
      setCurrentCursor(nextCursor);
    }
  };

  const handlePrevPage = () => {
    if (cursorHistory.length > 1) {
      const newHistory = [...cursorHistory];
      newHistory.pop();
      setCursorHistory(newHistory);
      setCurrentCursor(newHistory[newHistory.length - 1]);
    }
  };

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

  return (
    <div className="flex min-w-fit flex-col">
      <div className="sticky top-0 z-10 bg-background-primary">
        <div className="mb-4 flex items-center justify-between">
          <h2>Team Access Tokens</h2>
        </div>
      </div>
      <TeamAccessTokens
        accessTokens={tokens}
        onCreateToken={createTeamAccessToken}
        canCreate={canCreateTokens}
        canDelete={canDeleteTokens}
        isLoading={isLoading}
        hasMore={hasMore}
        currentPage={currentPage}
        canGoPrevious={cursorHistory.length > 1}
        onPreviousPage={handlePrevPage}
        onNextPage={handleNextPage}
      />
    </div>
  );
}
