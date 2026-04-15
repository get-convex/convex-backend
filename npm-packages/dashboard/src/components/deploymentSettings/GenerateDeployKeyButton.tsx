import { Button } from "@ui/Button";
import { toast } from "@common/lib/utils";
import { Checkbox } from "@ui/Checkbox";
import { Modal } from "@ui/Modal";
import { TextInput } from "@ui/TextInput";
import { CopyButton } from "@common/elements/CopyButton";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { Callout } from "@ui/Callout";
import { SegmentedControl } from "@ui/SegmentedControl";
import { useState } from "react";
import { PlusIcon } from "@radix-ui/react-icons";
import { DeploymentType as DeploymentTypeType } from "generatedApi";
import { usePostHog } from "hooks/usePostHog";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { HelpTooltip } from "@ui/HelpTooltip";

export type DeployKeyGenerationDisabledReason =
  | "CannotManageProd"
  | "LocalDeployment";

type OperationGroup = {
  label: string;
  operations: { key: string; label: string; description: string }[];
};

export function formatOperationName(name: string): string {
  return name
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .replace(/^(\S)/, (m) => m.toUpperCase())
    .replace(/\s(\S)/g, (m) => m.toLowerCase());
}

export const OPERATION_GROUPS: OperationGroup[] = [
  {
    label: "Deployment",
    operations: [
      {
        key: "Deploy",
        label: formatOperationName("Deploy"),
        description:
          "Allows deploying to this deployment. This includes updating code, the database schema, and auth configuration.",
      },
      {
        key: "PauseDeployment",
        label: formatOperationName("PauseDeployment"),
        description:
          "Allows pausing this deployment, blocking all functions from running, including scheduled functions and Cron jobs.",
      },
      {
        key: "UnpauseDeployment",
        label: formatOperationName("UnpauseDeployment"),
        description:
          "Allows unpausing this deployment, re-enabling functions, scheduled functions, and Cron jobs.",
      },
    ],
  },
  {
    label: "Environment variables",
    operations: [
      {
        key: "ViewEnvironmentVariables",
        label: formatOperationName("ViewEnvironmentVariables"),
        description:
          "Allows viewing all environment variables configured for this deployment.",
      },
      {
        key: "WriteEnvironmentVariables",
        label: formatOperationName("WriteEnvironmentVariables"),
        description:
          "Allows creating, updating, and deleting all environment variables in this deployment.",
      },
    ],
  },
  {
    label: "Data",
    operations: [
      {
        key: "ViewData",
        label: formatOperationName("ViewData"),
        description:
          "Allows viewing all data stored in this deployment, including data in tables, the database schema, scheduled functions, and file storage.",
      },
      {
        key: "WriteData",
        label: formatOperationName("WriteData"),
        description:
          "Allows writing to all data in this deployment, including updating data in tables, uploading and deleting files, canceling scheduled jobs, and exporting data with streaming export.",
      },
    ],
  },
  {
    label: "Functions",
    operations: [
      {
        key: "RunInternalQueries",
        label: formatOperationName("RunInternalQueries"),
        description:
          "Allows running internal queries defined in this deployment.",
      },
      {
        key: "RunInternalMutations",
        label: formatOperationName("RunInternalMutations"),
        description:
          "Allows running internal mutations defined in this deployment.",
      },
      {
        key: "RunInternalActions",
        label: formatOperationName("RunInternalActions"),
        description:
          "Allows running internal actions defined in this deployment.",
      },
      {
        key: "RunTestQuery",
        label: formatOperationName("RunTestQuery"),
        description:
          "Allows running custom test queries against this deployment.",
      },
      {
        key: "ActAsUser",
        label: formatOperationName("ActAsUser"),
        description:
          "Allows running functions assuming a specific user identity.",
      },
    ],
  },
  {
    label: "Backups",
    operations: [
      {
        key: "ViewBackups",
        label: formatOperationName("ViewBackups"),
        description: "Not yet implemented.",
      },
      {
        key: "CreateBackups",
        label: formatOperationName("CreateBackups"),
        description:
          "Allows exporting data with the Convex CLI. In the future, will also allow deploy keys to create cloud backups.",
      },
      {
        key: "DownloadBackups",
        label: formatOperationName("DownloadBackups"),
        description: "Allows downloading previously generated backups.",
      },
      {
        key: "DeleteBackups",
        label: formatOperationName("DeleteBackups"),
        description: "Not yet implemented.",
      },
      {
        key: "ImportBackups",
        label: formatOperationName("ImportBackups"),
        description:
          "Allows importing data with the Convex CLI and Streaming Import. In the future, will also allow deploy keys to restore from a cloud backup.",
      },
    ],
  },
  {
    label: "Monitoring",
    operations: [
      {
        key: "ViewLogs",
        label: formatOperationName("ViewLogs"),
        description: "Allows viewing function execution logs.",
      },
      {
        key: "ViewMetrics",
        label: formatOperationName("ViewMetrics"),
        description: "Allows viewing application metrics.",
      },
      {
        key: "ViewAuditLog",
        label: formatOperationName("ViewAuditLog"),
        description:
          "Allows viewing the deployment audit log, visible on the dashboard's history page.",
      },
    ],
  },
  {
    label: "Integrations",
    operations: [
      {
        key: "ViewIntegrations",
        label: formatOperationName("ViewIntegrations"),
        description:
          "Allows viewing integration configured for this deployment.",
      },
      {
        key: "WriteIntegrations",
        label: formatOperationName("WriteIntegrations"),
        description: "Allows configuring integrations for this deployment.",
      },
    ],
  },
];

type PermissionMode = "deploy" | "custom";

const PERMISSION_MODE_OPTIONS: {
  label: string;
  value: PermissionMode;
}[] = [
  { label: "Deploy only", value: "deploy" },
  { label: "Custom permissions", value: "custom" },
];

export type GenerateDeployKeyWithNameButtonProps = {
  disabledReason: DeployKeyGenerationDisabledReason | null;
  getAdminKey: (
    name: string,
    allowedOperations: string[] | undefined,
  ) => Promise<{ ok: true; adminKey: string } | { ok: false }>;
  deploymentType: DeploymentTypeType;
  showCustomPermissions?: boolean;
};

export function GenerateDeployKeyWithNameButton({
  disabledReason,
  getAdminKey,
  deploymentType,
  showCustomPermissions = true,
}: GenerateDeployKeyWithNameButtonProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [name, setName] = useState("");
  const [permissionMode, setPermissionMode] =
    useState<PermissionMode>("deploy");
  const [selectedOps, setSelectedOps] = useState<Set<string>>(() => new Set());
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const { capture } = usePostHog();
  const { scopedDeployKeys } = useLaunchDarkly();

  const handleClose = () => {
    setIsOpen(false);
    setCreatedKey(null);
    setName("");
    setPermissionMode("deploy");
    setSelectedOps(new Set());
  };

  return (
    <>
      {isOpen &&
        (createdKey ? (
          <Modal title="Deploy Key Created" onClose={handleClose}>
            <div className="flex flex-col gap-4">
              <p className="text-sm text-content-primary">
                Copy your new deploy key now. You won&apos;t be able to see it
                again.
              </p>
              <div className="flex items-center gap-2">
                <code className="min-w-0 flex-1 truncate rounded bg-background-tertiary px-2 py-1 text-sm">
                  {createdKey}
                </code>
                <CopyButton text={createdKey} />
              </div>
              <div className="flex justify-end">
                <Button onClick={handleClose}>Done</Button>
              </div>
            </div>
          </Modal>
        ) : (
          <Modal
            title="Create Deploy Key"
            onClose={handleClose}
            size={scopedDeployKeys && showCustomPermissions ? "md" : "sm"}
          >
            <form
              className="flex flex-col gap-3"
              onSubmit={async (e) => {
                e.preventDefault();
                setIsLoading(true);
                try {
                  const allowedOperations =
                    scopedDeployKeys && showCustomPermissions
                      ? permissionMode === "deploy"
                        ? ["Deploy"]
                        : Array.from(selectedOps)
                      : undefined;
                  const result = await getAdminKey(name, allowedOperations);
                  if (!result.ok) {
                    toast("error", "Error generating deploy key");
                    return;
                  }
                  setCreatedKey(result.adminKey);
                  capture("generated_deploy_key", {
                    type: deploymentType,
                  });
                } finally {
                  setIsLoading(false);
                }
              }}
            >
              <TextInput
                label="Name"
                id="name"
                autoFocus
                value={name}
                placeholder="Enter a memorable name for your deploy key"
                onChange={(event) => {
                  setName(event.target.value);
                }}
              />
              {scopedDeployKeys && showCustomPermissions && (
                <div className="flex flex-col gap-3">
                  <SegmentedControl
                    className="w-fit"
                    options={PERMISSION_MODE_OPTIONS}
                    value={permissionMode}
                    onChange={(value) => {
                      setPermissionMode(value);
                      if (value === "custom") {
                        setSelectedOps(new Set());
                      }
                    }}
                  />
                  {permissionMode === "deploy" ? (
                    <p className="text-xs text-content-secondary">
                      This key will able to deploy to this deployment. Deploying
                      includes updating code, the database schema, and auth
                      configuration.
                    </p>
                  ) : (
                    <>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="neutral"
                          size="xs"
                          onClick={() => {
                            const all = new Set(
                              OPERATION_GROUPS.flatMap((g) =>
                                g.operations.map((op) => op.key),
                              ),
                            );
                            setSelectedOps(all);
                          }}
                        >
                          All operations
                        </Button>
                        <Button
                          variant="neutral"
                          size="xs"
                          onClick={() => {
                            setSelectedOps(new Set());
                          }}
                        >
                          No operations
                        </Button>
                      </div>
                      <div className="scrollbar max-h-[60dvh] overflow-y-auto">
                        <div className="flex flex-col gap-3">
                          {OPERATION_GROUPS.map((group) => (
                            <div key={group.label}>
                              <div className="mb-1 text-sm font-semibold text-content-secondary">
                                {group.label}
                              </div>
                              <div className="grid grid-cols-[repeat(auto-fill,minmax(12rem,1fr))] gap-x-4 gap-y-1">
                                {group.operations.map((op) => (
                                  <label
                                    key={op.key}
                                    htmlFor={`op-${op.key}`}
                                    className="flex cursor-pointer items-center gap-2 rounded px-1 py-1 text-xs hover:bg-background-secondary"
                                  >
                                    <Checkbox
                                      id={`op-${op.key}`}
                                      checked={selectedOps.has(op.key)}
                                      onChange={() => {
                                        setSelectedOps((prev) => {
                                          const next = new Set(prev);
                                          if (next.has(op.key)) {
                                            next.delete(op.key);
                                          } else {
                                            next.add(op.key);
                                          }
                                          return next;
                                        });
                                      }}
                                    />
                                    {op.label}
                                    <HelpTooltip>{op.description}</HelpTooltip>
                                  </label>
                                ))}
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    </>
                  )}
                </div>
              )}
              <div className="flex items-center justify-end gap-2">
                {scopedDeployKeys &&
                  showCustomPermissions &&
                  permissionMode === "custom" &&
                  selectedOps.size === 0 && (
                    <span className="text-xs text-content-errorSecondary">
                      Select at least one operation
                    </span>
                  )}
                <Button
                  className="w-fit"
                  type="submit"
                  disabled={
                    disabledReason !== null ||
                    name.trim() === "" ||
                    (scopedDeployKeys &&
                      showCustomPermissions &&
                      permissionMode === "custom" &&
                      selectedOps.size === 0)
                  }
                  loading={isLoading}
                >
                  Create
                </Button>
              </div>
            </form>
          </Modal>
        ))}
      <Button
        disabled={disabledReason !== null}
        tip={
          disabledReason === "CannotManageProd"
            ? "You do not have permission to generate a production deploy key."
            : disabledReason === "LocalDeployment"
              ? "You cannot generate deploy keys for a local deployment."
              : undefined
        }
        onClick={() => setIsOpen(true)}
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
          disabled={disabledReason !== null}
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
          loading={isLoading}
          icon={<PlusIcon />}
        >
          {getGenerateButtonText(deploymentType)}
        </Button>
      )}
    </>
  );
}

const DEPLOY_KEY_GENERATION_DISABLED_REASONS = {
  CannotManageProd:
    "You do not have permission to generate a production deploy key.",
  LocalDeployment: "You cannot generate deploy keys for a local deployment.",
} as const;

function getGenerateButtonText(_deploymentType: DeploymentTypeType) {
  return "Create Deploy Key";
}
