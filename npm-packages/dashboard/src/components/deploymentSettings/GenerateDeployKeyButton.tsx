import { Button, Spinner, toast, Callout } from "dashboard-common";
import { useState } from "react";
import { PlusIcon } from "@radix-ui/react-icons";
import { DeploymentType as DeploymentTypeType } from "generatedApi";
import { CopyTextButton } from "elements/CopyTextButton";
import { Modal } from "elements/Modal";
import startCase from "lodash/startCase";
import { TextInput } from "elements/TextInput";

export type DeployKeyGenerationDisabledReason =
  | "CannotManageProd"
  | "LocalDeployment";

const DEPLOY_KEY_GENERATION_DISABLED_REASONS = {
  CannotManageProd:
    "You do not have permission to generate a production deploy key.",
  LocalDeployment: "You cannot generate deploy keys for a local deployment.",
} as const;

export type GenerateDeployKeyWithNameButtonProps = {
  onCreateAccessToken?: (token: string) => void;
  disabledReason: DeployKeyGenerationDisabledReason | null;
  getAdminKey: (
    name: string,
  ) => Promise<{ ok: true; adminKey: string } | { ok: false }>;
  deploymentType: DeploymentTypeType;
};

export function GenerateDeployKeyWithNameButton({
  onCreateAccessToken,
  disabledReason,
  getAdminKey,
  deploymentType,
}: GenerateDeployKeyWithNameButtonProps) {
  const [name, setName] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  return (
    <>
      {name !== null && (
        <Modal
          title={`Create ${startCase(deploymentType)} Deploy Key`}
          onClose={() => setName(null)}
        >
          <div className="flex flex-col gap-2">
            Enter a name for your deploy key:
            <form
              className="flex gap-2"
              onSubmit={async (e) => {
                e.preventDefault();
                setIsLoading(true);
                try {
                  const result = await getAdminKey(name);
                  if (!result.ok) {
                    toast("error", "Error generating deploy key");
                    return;
                  }
                  onCreateAccessToken && onCreateAccessToken(result.adminKey);
                  setName(null);
                } finally {
                  setIsLoading(false);
                }
              }}
            >
              <TextInput
                id="name"
                autoFocus
                labelHidden
                value={name}
                placeholder="Deploy Key Name"
                onChange={(event) => {
                  setName(event.target.value);
                }}
              />
              <Button
                type="submit"
                disabled={disabledReason !== null || isLoading}
                icon={isLoading && <Spinner />}
              >
                Save
              </Button>
            </form>
          </div>
        </Modal>
      )}
      <Button
        disabled={disabledReason !== null}
        tip={
          disabledReason === "CannotManageProd"
            ? "You do not have permission to generate a production deploy key."
            : disabledReason === "LocalDeployment"
              ? "You cannot generate deploy keys for a local deployment."
              : undefined
        }
        onClick={() => {
          setName("");
        }}
        icon={<PlusIcon />}
      >
        {getGenerateButtonText(deploymentType)}
      </Button>
    </>
  );
}

type GenerateDeployKeyButtonProps = {
  deploymentType: DeploymentTypeType;
  getAdminKey: () => Promise<{ ok: true; adminKey: string } | { ok: false }>;
  disabledReason: DeployKeyGenerationDisabledReason | null;
};

export function GenerateDeployKeyButton({
  deploymentType,
  getAdminKey,
  disabledReason,
}: GenerateDeployKeyButtonProps) {
  const [isLoading, setIsLoading] = useState(false);

  const [deployKey, setDeployKey] = useState<string | null>(null);

  return (
    <>
      {deployKey ? (
        <div className="flex max-w-[32rem] flex-col gap-3">
          <Callout variant="instructions">
            This key enables reading and writing data to your deployment without
            needing to log in, so it should not be shared or committed to git.
          </Callout>
          <CopyTextButton
            text={deployKey}
            className="block max-w-[30rem] truncate font-mono text-sm font-normal"
          />
        </div>
      ) : (
        <Button
          disabled={disabledReason !== null || isLoading}
          tip={
            disabledReason === null
              ? undefined
              : DEPLOY_KEY_GENERATION_DISABLED_REASONS[disabledReason]
          }
          onClick={async () => {
            setIsLoading(true);
            try {
              if (deployKey === null) {
                const result = await getAdminKey();
                if (!result.ok) {
                  toast("error", "Error generating deploy key");
                  return;
                }
                setDeployKey(result.adminKey);
              }
            } finally {
              setIsLoading(false);
            }
          }}
          className="my-2 mr-auto"
          icon={isLoading ? <Spinner /> : <PlusIcon />}
        >
          {getGenerateButtonText(deploymentType)}
        </Button>
      )}
    </>
  );
}

function getGenerateButtonText(deploymentType: DeploymentTypeType) {
  switch (deploymentType) {
    case "prod":
      return "Generate Production Deploy Key";
    case "dev":
      return "Generate Development Deploy Key";
    case "preview": {
      return "Generate Preview Deploy Key";
    }
    default: {
      const _typecheck: never = deploymentType;
      return "Generate Deploy Key";
    }
  }
}
