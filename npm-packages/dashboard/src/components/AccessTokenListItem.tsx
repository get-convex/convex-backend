import { EyeOpenIcon, EyeNoneIcon, Cross2Icon } from "@radix-ui/react-icons";
import { AccessTokenListKind, useDeleteAccessToken } from "api/accessTokens";
import { Button } from "@ui/Button";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { TeamAccessTokenResponse } from "generatedApi";
import { useCurrentTeam, useTeamMembers } from "api/teams";
import { useEffect, useState } from "react";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { usePostHog } from "hooks/usePostHog";
import { permissionDeniedTip } from "elements/permissionDeniedTip";
import type { RoleStatementAction } from "@convex-dev/platform/managementApi";

const DELETE_ACTION_BY_KIND: Record<AccessTokenListKind, RoleStatementAction> =
  {
    team: "team:token:delete",
    project: "project:token:delete",
    deployment: "deployment:token:delete",
  };

export function AccessTokenListItem({
  token,
  identifier,
  tokenPrefix,
  kind,
  shouldShow,
  showMemberName = true,
  canDelete,
}: {
  token: TeamAccessTokenResponse;
  identifier: string;
  tokenPrefix?: string;
  kind: AccessTokenListKind;
  shouldShow: boolean;
  showMemberName?: boolean;
  // `undefined` means the caller hasn't wired a permission check yet —
  // leave the button enabled rather than silently disabling it. Pass a
  // boolean to actually gate it.
  canDelete?: boolean | undefined;
}) {
  const team = useCurrentTeam();
  const members = useTeamMembers(team?.id);
  const deleteAccessToken = useDeleteAccessToken(identifier, kind);
  const [showToken, setShowToken] = useState(shouldShow);
  useEffect(() => {
    if (shouldShow) {
      setShowToken(shouldShow);
    }
  }, [shouldShow]);
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const { capture } = usePostHog();

  const member = showMemberName
    ? members?.find((m) => m.id === token.creator)
    : null;

  return (
    <div key={token.accessToken} className="flex w-full flex-col">
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2">
        <div>{token.name}</div>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex flex-col items-end">
            {token.lastUsedTime !== null && token.lastUsedTime !== undefined ? (
              <TimestampDistance
                prefix="Last used "
                date={new Date(token.lastUsedTime)}
              />
            ) : (
              <div className="text-xs text-content-secondary">Never used</div>
            )}
            <div className="flex gap-1">
              <TimestampDistance
                prefix="Created "
                date={new Date(token.creationTime)}
              />
              {showMemberName && (
                <div className="flex items-center gap-1 text-xs text-content-secondary">
                  by{" "}
                  {member ? (
                    <TeamMemberLink
                      memberId={token.creator}
                      name={member?.name || member?.email}
                    />
                  ) : (
                    "Unknown member"
                  )}
                </div>
              )}
            </div>
            {token.expiresAt !== null && token.expiresAt !== undefined && (
              <TimestampDistance
                prefix="Expires "
                date={new Date(token.expiresAt)}
                className="text-left text-content-errorSecondary"
              />
            )}
          </div>
          <div className="flex gap-2">
            <Button
              variant="neutral"
              icon={showToken ? <EyeNoneIcon /> : <EyeOpenIcon />}
              onClick={() => {
                setShowToken(!showToken);
              }}
            >
              {showToken ? "Hide" : "Show"}
            </Button>
            <Button
              variant="danger"
              icon={<Cross2Icon />}
              onClick={() => {
                setShowDeleteConfirmation(true);
              }}
              disabled={canDelete === false}
              tip={
                canDelete === false
                  ? permissionDeniedTip(
                      "You do not have permission to delete this access token.",
                      DELETE_ACTION_BY_KIND[kind],
                    )
                  : undefined
              }
            >
              Delete
            </Button>
          </div>
        </div>
      </div>
      <div className="mb-2 flex items-center gap-1">
        {showToken && (
          <div className="mt-1 flex flex-col gap-1">
            <CopyTextButton
              text={
                tokenPrefix
                  ? `${tokenPrefix}|${token.serializedAccessToken}`
                  : token.serializedAccessToken
              }
              className="block max-w-120 truncate font-mono text-sm font-normal"
            />
          </div>
        )}
      </div>
      {showDeleteConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDeleteConfirmation(false);
          }}
          onConfirm={async () => {
            await deleteAccessToken({ accessToken: token.accessToken });
            if (tokenPrefix) {
              const type = tokenPrefix.split(":")[0] ?? tokenPrefix;
              capture("deleted_deploy_key", { type });
            }
          }}
          confirmText="Delete"
          dialogTitle="Delete Access Token"
          dialogBody={
            <>
              Are you sure you want to delete:{" "}
              <span className="font-semibold">{token.name}</span>?
            </>
          }
        />
      )}
    </div>
  );
}
