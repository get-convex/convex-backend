import { useState } from "react";
import { TeamAccessTokenResponse } from "generatedApi";
import { AccessTokenListKind } from "api/accessTokens";

import { LoadingTransition } from "@ui/Loading";
import {
  GenerateDeployKeyWithNameButton,
  GenerateDeployKeyWithNameButtonProps,
  DeployKeyGenerationDisabledReason,
} from "./GenerateDeployKeyButton";
import { DeploymentAccessTokenListItem } from "./DeploymentAccessTokenListItem";

export function DeploymentAccessTokenList({
  identifier,
  tokenPrefix,
  accessTokens,
  kind,
  disabledReason,
  buttonProps,
  header,
  description,
}: {
  identifier: string;
  tokenPrefix: string;
  accessTokens: TeamAccessTokenResponse[] | undefined;
  kind: AccessTokenListKind;
  disabledReason: DeployKeyGenerationDisabledReason | null;
  buttonProps: Omit<
    GenerateDeployKeyWithNameButtonProps,
    "onCreateAccessToken"
  >;
  header: string;
  description: React.ReactNode;
}) {
  const [latestToken, setLatestToken] = useState<string | null>(null);
  return (
    <>
      <div className="mb-2 flex w-full items-center justify-between">
        <h4>{header}</h4>
        <GenerateDeployKeyWithNameButton
          {...buttonProps}
          onCreateAccessToken={setLatestToken}
        />
      </div>
      {description}
      {disabledReason === null && (
        <LoadingTransition
          loadingProps={{ fullHeight: false, className: "h-14 w-full" }}
        >
          {accessTokens && (
            <div className="flex w-full flex-col divide-y">
              {accessTokens.length > 0 ? (
                accessTokens
                  ?.sort((a, b) => b.creationTime - a.creationTime)
                  .map((token) => (
                    <DeploymentAccessTokenListItem
                      token={token}
                      identifier={identifier}
                      tokenPrefix={tokenPrefix}
                      kind={kind}
                      key={token.accessToken}
                      shouldShow={
                        !!latestToken &&
                        latestToken.endsWith(token.serializedAccessToken)
                      }
                    />
                  ))
              ) : (
                <div className="my-6 flex w-full justify-center text-content-secondary">
                  There are no tokens here yet.
                </div>
              )}
            </div>
          )}
        </LoadingTransition>
      )}
    </>
  );
}
