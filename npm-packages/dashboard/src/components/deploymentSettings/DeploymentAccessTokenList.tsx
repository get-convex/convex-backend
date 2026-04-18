import { PlatformDeployKeyResponse } from "@convex-dev/platform/managementApi";

import { LoadingTransition } from "@ui/Loading";
import { DeployKeyListItem } from "components/DeployKeyListItem";
import {
  GenerateDeployKeyWithNameButton,
  GenerateDeployKeyWithNameButtonProps,
  DeployKeyGenerationDisabledReason,
} from "./GenerateDeployKeyButton";

export function DeploymentAccessTokenList({
  deploymentType,
  onDelete,
  deployKeys,
  disabledReason,
  buttonProps,
  header,
  description,
  headingLevel = "h4",
}: {
  deploymentType: string;
  onDelete: (args: { id: string }) => Promise<unknown>;
  deployKeys: PlatformDeployKeyResponse[] | undefined;
  disabledReason: DeployKeyGenerationDisabledReason | null;
  buttonProps: GenerateDeployKeyWithNameButtonProps;
  header: string;
  description: React.ReactNode;
  headingLevel?: "h3" | "h4";
}) {
  const HeadingTag = (headingLevel ?? "h4") as keyof JSX.IntrinsicElements;
  return (
    <>
      <div className="mb-2 flex w-full items-center justify-between">
        <HeadingTag>{header}</HeadingTag>
        <GenerateDeployKeyWithNameButton {...buttonProps} />
      </div>
      {description}
      {disabledReason === null && (
        <LoadingTransition
          loadingProps={{ fullHeight: false, className: "h-14 w-full" }}
        >
          {deployKeys && (
            <div className="flex w-full flex-col divide-y">
              {deployKeys.length > 0 ? (
                deployKeys
                  ?.sort((a, b) => b.creationTime - a.creationTime)
                  .map((deployKey) => (
                    <DeployKeyListItem
                      deployKey={deployKey}
                      deploymentType={deploymentType}
                      onDelete={onDelete}
                      key={deployKey.name}
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
