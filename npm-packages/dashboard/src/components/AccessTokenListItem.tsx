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

export function AccessTokenListItem({
  token,
  identifier,
  tokenPrefix,
  kind,
  shouldShow,
  showMemberName = true,
}: {
  token: TeamAccessTokenResponse;
  identifier: string;
  tokenPrefix?: string;
  kind: AccessTokenListKind;
  shouldShow: boolean;
  showMemberName?: boolean;
}) {
  const team = useCurrentTeam();
  const members = useTeamMembers(team?.id);
  const deleteAccessToken = useDeleteAccessToken(identifier, kind);
  const [showToken, setShowToken] = useState(shouldShow);
  useEffect(() => {
    shouldShow && setShowToken(shouldShow);
  }, [shouldShow]);
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);

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
              className="block max-w-[30rem] truncate font-mono text-sm font-normal"
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
