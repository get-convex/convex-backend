import { AppAccessTokenResponse } from "generatedApi";
import { Sheet } from "@ui/Sheet";
import { LoadingTransition } from "@ui/Loading";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { Button } from "@ui/Button";
import { Cross2Icon } from "@radix-ui/react-icons";
import React, { useState } from "react";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";

export function AuthorizedApplications({
  accessTokens,
  explainer,
  onRevoke,
}: {
  accessTokens: AppAccessTokenResponse[] | undefined;
  explainer: React.ReactNode;
  onRevoke: (token: AppAccessTokenResponse) => Promise<void>;
}) {
  return (
    <Sheet>
      {explainer}
      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "h-14 w-full" }}
      >
        {accessTokens !== undefined && (
          <div className="mt-2 flex w-full flex-col gap-2 divide-y">
            {accessTokens.length ? (
              accessTokens.map((token) => (
                <AuthorizedApplicationListItem
                  key={token.name}
                  token={token}
                  onRevoke={onRevoke}
                />
              ))
            ) : (
              <div className="my-6 flex w-full justify-center text-content-secondary">
                You have not authorized any applications yet.
              </div>
            )}
          </div>
        )}
      </LoadingTransition>
    </Sheet>
  );
}

function AuthorizedApplicationListItem({
  token,
  onRevoke,
}: {
  token: AppAccessTokenResponse;
  onRevoke: (token: AppAccessTokenResponse) => Promise<void>;
}) {
  const [showConfirmation, setShowConfirmation] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  return (
    <div className="flex w-full flex-col pb-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>{token.appName}</div>
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
            <TimestampDistance
              prefix="Created "
              date={new Date(token.creationTime)}
            />
          </div>
          <Button
            variant="danger"
            icon={<Cross2Icon />}
            onClick={() => setShowConfirmation(true)}
            loading={isDeleting}
          >
            Revoke
          </Button>
        </div>
      </div>
      {showConfirmation && (
        <ConfirmationDialog
          dialogTitle={`Revoke access for ${token.appName}`}
          dialogBody="Are you sure you want to revoke access for this application?"
          confirmText="Revoke"
          onClose={() => setShowConfirmation(false)}
          onConfirm={async () => {
            setIsDeleting(true);
            try {
              await onRevoke(token);
            } finally {
              setIsDeleting(false);
            }
          }}
        />
      )}
    </div>
  );
}
