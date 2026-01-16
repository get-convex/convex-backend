import { useContext, useEffect, useMemo, useState } from "react";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import {
  DeploymentInfo,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import Link from "next/link";
import { Loading } from "@ui/Loading";
import { Tooltip } from "@ui/Tooltip";
import {
  QuestionMarkCircledIcon,
  CheckIcon,
  ExclamationTriangleIcon,
  Cross2Icon,
  EyeOpenIcon,
  EyeNoneIcon,
  ArrowRightIcon,
  ExternalLinkIcon,
} from "@radix-ui/react-icons";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { EnvironmentVariable } from "system-udfs/convex/_system/frontend/common";
import { Callout } from "@ui/Callout";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { toast } from "@common/lib/utils";
import { useUpdateEnvVars } from "@common/features/settings/lib/api";
import { ExpandableActionSection } from "./ExpandableActionSection";

type WorkOSEnvVars = {
  clientId: string | null;
  environmentId: string | null;
  apiKey: string | null;
};

type ProvisionedEnvironment = {
  workosEnvironmentId: string;
  workosEnvironmentName: string;
  workosClientId: string;
  workosApiKey?: string; // Optional since backend might not always return it
  workosTeamId: string;
  isProduction: boolean;
};

type EnvVarChange = {
  name: string;
  currentValue: string | null;
  newValue: string;
};

// Helper to extract error info from unknown errors (typically API errors with code/message)
function getErrorInfo(error: unknown): { code: string; message: string } {
  if (error instanceof Error) {
    return {
      code: (error as Error & { code?: string }).code ?? "",
      message: error.message,
    };
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return {
      code: "code" in error && typeof error.code === "string" ? error.code : "",
      message: error.message,
    };
  }
  return { code: "", message: String(error) };
}

function InfoTooltip({
  items,
}: {
  items: Array<{ label: string; value: string; isSecret?: boolean }>;
}) {
  const [showApiKey, setShowApiKey] = useState(false);

  return (
    <Tooltip
      maxWidthClassName="max-w-md"
      tip={
        <div className="flex flex-col gap-3 p-1 text-left">
          {items.map((item) => (
            <div key={item.label}>
              <div className="mb-1.5 flex items-center gap-1.5">
                <span className="text-xs font-semibold">{item.label}</span>
                {item.isSecret && (
                  <Button
                    type="button"
                    onClick={() => setShowApiKey(!showApiKey)}
                    variant="neutral"
                    size="sm"
                    inline
                    icon={showApiKey ? <EyeNoneIcon /> : <EyeOpenIcon />}
                    aria-label={showApiKey ? "Hide value" : "Show value"}
                  />
                )}
              </div>
              <div className="max-w-full overflow-hidden">
                {item.isSecret && !showApiKey ? (
                  <span className="font-mono text-xs text-content-secondary">
                    •••••••••••••••
                  </span>
                ) : (
                  <CopyTextButton
                    text={item.value}
                    className="w-full font-mono text-xs [&_span]:break-all [&_span]:whitespace-normal"
                  />
                )}
              </div>
            </div>
          ))}
        </div>
      }
    >
      <QuestionMarkCircledIcon className="inline text-content-tertiary" />
    </Tooltip>
  );
}

function EnvVarChangeRow({ change }: { change: EnvVarChange }) {
  const [showValues, setShowValues] = useState(false);
  const isNew = change.currentValue === null;
  const isSecret = change.name === "WORKOS_API_KEY";
  const shouldShowCurrentValue = !isSecret || showValues;
  const shouldShowNewValue = !isSecret || showValues;

  return (
    <div className="flex flex-col gap-1.5 rounded border bg-background-secondary p-3 text-xs">
      <div className="flex items-center gap-2">
        <div className="font-mono font-semibold text-content-primary">
          {change.name}
        </div>
        {isSecret && (
          <Button
            type="button"
            onClick={() => setShowValues(!showValues)}
            variant="neutral"
            size="sm"
            inline
            icon={showValues ? <EyeNoneIcon /> : <EyeOpenIcon />}
            tip={showValues ? "Hide value" : "Show value"}
          />
        )}
      </div>

      <div className="flex items-center gap-2 font-mono text-xs">
        {isNew ? (
          <>
            <div className="flex-1 text-content-secondary">Not set</div>
            <ArrowRightIcon className="h-3 w-3 flex-shrink-0 text-content-secondary" />
            <div className="flex-1 overflow-x-auto text-content-success">
              <div className="inline-block min-w-0 whitespace-nowrap">
                {shouldShowNewValue ? change.newValue : "•••••••••"}
              </div>
            </div>
          </>
        ) : (
          <>
            <div className="flex-1 overflow-x-auto text-content-secondary">
              <div className="inline-block min-w-0 whitespace-nowrap line-through">
                {shouldShowCurrentValue ? change.currentValue : "•••••••••"}
              </div>
            </div>
            <ArrowRightIcon className="h-3 w-3 flex-shrink-0 text-content-secondary" />
            <div className="flex-1 overflow-x-auto text-content-success">
              <div className="inline-block min-w-0 whitespace-nowrap">
                {shouldShowNewValue ? change.newValue : "•••••••••"}
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function getWorkOSNotSupportedReason(
  deployment: ReturnType<DeploymentInfo["useCurrentDeployment"]>,
): string | undefined {
  if (!deployment) {
    return undefined;
  }
  const { deploymentType, kind } = deployment;

  // Only local dev deployments are not supported
  if (deploymentType === "dev" && kind === "local") {
    return "Automatic environment creation for local deployments is not supported.";
  }

  // Cloud dev, prod, and preview are all supported
  return undefined;
}

function useRelevantEnvVars() {
  // Fetch deployment-level environment variables
  const environmentVariables: undefined | Array<EnvironmentVariable> = useQuery(
    udfs.listEnvironmentVariables.default,
    {},
  );

  // Extract WorkOS environment variables
  const workosEnvVars = useMemo(() => {
    if (!environmentVariables) return null;

    const clientId = environmentVariables.find(
      (envVar) => envVar.name === "WORKOS_CLIENT_ID",
    )?.value;
    const environmentId = environmentVariables.find(
      (envVar) => envVar.name === "WORKOS_ENVIRONMENT_ID",
    )?.value;
    const apiKey = environmentVariables.find(
      (envVar) => envVar.name === "WORKOS_API_KEY",
    )?.value;

    return {
      clientId: clientId || null,
      environmentId: environmentId || null,
      apiKey: apiKey || null,
    };
  }, [environmentVariables]);

  return workosEnvVars;
}

function InviteTeamMemberSection({ teamId }: { teamId?: number }) {
  const { workOSOperations } = useContext(DeploymentInfoContext);

  const [showInviteForm, setShowInviteForm] = useState(false);
  const [selectedEmail, setSelectedEmail] = useState<string>("");
  const [isInviting, setIsInviting] = useState(false);

  const eligibleEmailsData = workOSOperations.useWorkOSInvitationEligibleEmails(
    teamId?.toString(),
  );
  const inviteMutation = workOSOperations.useInviteWorkOSTeamMember();

  const handleInvite = async () => {
    if (!teamId || !selectedEmail) return;

    setIsInviting(true);
    try {
      await inviteMutation({ teamId, email: selectedEmail });
      setSelectedEmail("");
      setShowInviteForm(false);
    } catch (error) {
      console.error("Failed to send WorkOS invitation:", error);
      // Check if this is the "already a member of another workspace" error
      const { code: errorCode, message: errorMessage } = getErrorInfo(error);
      if (
        errorCode === "WorkosUserAlreadyInWorkspace" ||
        errorMessage.includes("already a member of another team")
      ) {
        // This error indicates the user should already be able to log in
        toast(
          "error",
          `${selectedEmail} is already a member of another WorkOS workspace. Try another email address.`,
        );
      } else {
        // Show a generic error toast for other errors
        toast(
          "error",
          errorMessage || "Failed to send invitation. Please try again.",
        );
      }
    } finally {
      setIsInviting(false);
    }
  };

  if (!eligibleEmailsData || eligibleEmailsData.eligibleEmails.length === 0) {
    return null;
  }

  if (!showInviteForm) {
    return (
      <div className="flex flex-col gap-2">
        <p className="text-sm text-content-secondary">
          Invite a Convex email address to access this WorkOS workspace.
        </p>
        <div>
          <Button size="sm" onClick={() => setShowInviteForm(true)}>
            Invite to WorkOS
          </Button>
        </div>
      </div>
    );
  }

  const options = eligibleEmailsData.eligibleEmails.map((email) => ({
    value: email,
    label: email,
  }));

  return (
    <div className="flex flex-col gap-2 rounded-sm border p-3">
      <div className="flex flex-col gap-1">
        <div className="text-sm font-semibold text-content-primary">
          Send invite email to WorkOS
        </div>
        <p className="text-xs text-content-secondary">
          Send or re-send an invitation email to one of your verified emails to
          access the WorkOS dashboard for this workspace (invitations expire
          after some time).
        </p>
        {eligibleEmailsData.adminEmail && (
          <p className="text-xs text-content-tertiary">
            Workspace Admin: {eligibleEmailsData.adminEmail}
          </p>
        )}
      </div>

      <div className="flex flex-col gap-2">
        <Combobox
          label="Select email"
          options={options}
          selectedOption={selectedEmail}
          setSelectedOption={(value) => setSelectedEmail(value || "")}
          disableSearch={options.length <= 5}
          allowCustomValue={false}
          buttonClasses="w-full bg-inherit"
        />

        <div className="flex gap-2">
          <Button
            size="sm"
            onClick={handleInvite}
            disabled={!selectedEmail}
            loading={isInviting}
          >
            Send Invite
          </Button>
          <Button
            size="sm"
            variant="neutral"
            onClick={() => {
              setShowInviteForm(false);
              setSelectedEmail("");
            }}
          >
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}

function ProvisionWorkOSTeamSection({
  teamId,
  onTeamCreated,
}: {
  teamId?: number;
  onTeamCreated?: () => void;
}) {
  const { workOSOperations } = useContext(DeploymentInfoContext);

  const [showEmailSelection, setShowEmailSelection] = useState(false);
  const [selectedEmail, setSelectedEmail] = useState<string>("");
  const [isProvisioning, setIsProvisioning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const availableEmailsData = workOSOperations.useAvailableWorkOSTeamEmails();
  const provisionMutation = workOSOperations.useProvisionWorkOSTeam(
    teamId?.toString(),
  );

  const handleProvision = async () => {
    if (!teamId || !selectedEmail) return;

    setIsProvisioning(true);
    setError(null);
    try {
      await provisionMutation({ teamId, email: selectedEmail });
      setShowEmailSelection(false);
      setSelectedEmail("");
      if (onTeamCreated) {
        onTeamCreated();
      }
    } catch (e) {
      console.error("Failed to provision WorkOS team:", e);
      // Check for the "already exists" error
      const { code, message } = getErrorInfo(e);
      if (
        message.includes("already") ||
        code === "WorkosAccountAlreadyExistsWithThisEmail"
      ) {
        setError(
          `A WorkOS team already exists with ${selectedEmail}. Please select a different email address.`,
        );
      } else {
        setError("Failed to create WorkOS workspace. Please try again.");
      }
      setIsProvisioning(false);
    }
  };

  if (!availableEmailsData) {
    return <Loading />;
  }

  const hasAvailableEmails = availableEmailsData.availableEmails.length > 0;
  const hasUsedEmails = availableEmailsData.usedEmails.length > 0;

  if (!showEmailSelection) {
    return (
      <div className="flex flex-col gap-2">
        <p className="text-sm text-content-secondary">
          No WorkOS workspace is linked to this team. Create a WorkOS workspace
          to enable automatic AuthKit environment provisioning.
        </p>
        <div>
          <Button
            size="sm"
            onClick={() => setShowEmailSelection(true)}
            disabled={!hasAvailableEmails}
            tip={
              hasAvailableEmails
                ? undefined
                : "All your verified emails are already used for other WorkOS teams"
            }
          >
            Create WorkOS Workspace
          </Button>
        </div>
      </div>
    );
  }

  const allEmails = [
    ...availableEmailsData.availableEmails,
    ...availableEmailsData.usedEmails,
  ];

  const emailOptions = allEmails.map((email) => {
    const isUsed = availableEmailsData.usedEmails.includes(email);
    return {
      value: email,
      label: isUsed ? `${email} (already used for another WorkOS team)` : email,
    };
  });

  return (
    <div className="flex flex-col gap-3 rounded-sm border p-3">
      <div className="flex flex-col gap-1">
        <div className="text-sm font-semibold text-content-primary">
          Create WorkOS Workspace
        </div>
        <p className="text-xs text-content-secondary">
          Select one of your verified email addresses to use as the admin for
          the new WorkOS workspace.
        </p>
        {hasUsedEmails && (
          <p className="text-xs text-content-tertiary">
            {availableEmailsData.usedEmails.length} of your email
            {availableEmailsData.usedEmails.length > 1 ? "s are" : " is"}{" "}
            already used for other teams and cannot be selected.
          </p>
        )}
      </div>

      {error && (
        <Callout variant="error">
          <p className="text-xs">{error}</p>
        </Callout>
      )}

      <div className="flex flex-col gap-2">
        <Combobox
          label="Admin email address"
          options={emailOptions.filter(
            (opt) => !availableEmailsData.usedEmails.includes(opt.value),
          )}
          selectedOption={selectedEmail}
          setSelectedOption={(value) => setSelectedEmail(value || "")}
          disableSearch={emailOptions.length <= 5}
          allowCustomValue={false}
          buttonClasses="w-full bg-inherit"
        />

        <div className="flex gap-2">
          <Button
            size="sm"
            onClick={handleProvision}
            disabled={!selectedEmail}
            loading={isProvisioning}
          >
            Create Workspace
          </Button>
          <Button
            size="sm"
            variant="neutral"
            onClick={() => {
              setShowEmailSelection(false);
              setError(null);
              setSelectedEmail("");
            }}
          >
            Cancel
          </Button>
        </div>
      </div>

      <p className="text-xs text-content-tertiary">
        Need to use a different email?{" "}
        <Link
          href="/profile"
          className="text-content-link hover:underline"
          target="_blank"
        >
          Add and verify an email in your profile
        </Link>
      </p>
    </div>
  );
}

function WorkOSTeamSection({
  workosTeam,
  environment,
  teamId,
  showCongratulations = false,
  onTeamCreated,
}: {
  workosTeam:
    | {
        workosTeamId: string;
        workosTeamName: string;
        workosAdminEmail: string;
      }
    | null
    | undefined;
  environment: {
    workosEnvironmentId: string;
    workosEnvironmentName: string;
    workosClientId: string;
    workosTeamId: string;
  } | null;
  teamId?: number;
  showCongratulations?: boolean;
  onTeamCreated?: () => void;
}) {
  const { workOSOperations } = useContext(DeploymentInfoContext);
  const [isDisconnecting, setIsDisconnecting] = useState(false);
  const [showSuccessMessage, setShowSuccessMessage] = useState(false);

  // Sync showCongratulations prop to local state (prop changes when team is created)
  useEffect(() => {
    if (showCongratulations) {
      setShowSuccessMessage(true);
    }
  }, [showCongratulations]);

  const teamHealthData = workOSOperations.useWorkOSTeamHealth(
    teamId?.toString(),
  );
  const disconnectMutation = workOSOperations.useDisconnectWorkOSTeam(
    teamId?.toString(),
  );

  const handleDisconnect = async () => {
    if (!teamId || !workosTeam) return;

    setIsDisconnecting(true);
    try {
      await disconnectMutation({ teamId });
    } catch (error) {
      console.error("Failed to disconnect WorkOS team:", error);
      const { message } = getErrorInfo(error);
      toast(
        "error",
        message || "Failed to disconnect WorkOS workspace. Please try again.",
      );
    } finally {
      setIsDisconnecting(false);
    }
  };

  if (!workosTeam) {
    return (
      <ProvisionWorkOSTeamSection
        teamId={teamId}
        onTeamCreated={onTeamCreated}
      />
    );
  }

  const teamInfo = teamHealthData?.data?.teamProvisioned
    ? teamHealthData.data.teamInfo
    : null;
  const canProvisionProduction = teamInfo?.productionState === "active";
  const isLoadingHealth =
    teamHealthData && !teamHealthData.data && !teamHealthData.error;

  return (
    <div className="flex flex-col gap-3">
      {/* Success message for newly created WorkOS team */}
      {showSuccessMessage && !environment && (
        <Callout variant="success">
          <div className="flex flex-col gap-2">
            <div className="font-semibold">
              Congratulations! Your WorkOS workspace has been created.
            </div>
            <div className="text-sm">
              <p className="mb-2">Next steps:</p>
              <ol className="list-inside list-decimal space-y-1">
                <li>
                  Check your email ({workosTeam.workosAdminEmail}) for an
                  invitation to the WorkOS dashboard
                </li>
                <li>Create AuthKit environments for your deployments above</li>
                <li>
                  Configure your authentication settings in the{" "}
                  <a
                    href={`https://dashboard.workos.com/${workosTeam.workosTeamId}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-content-link underline"
                  >
                    WorkOS dashboard
                  </a>
                </li>
              </ol>
            </div>
            <Button
              size="xs"
              variant="neutral"
              onClick={() => setShowSuccessMessage(false)}
            >
              Dismiss
            </Button>
          </div>
        </Callout>
      )}

      <div className="flex flex-col gap-2">
        <p className="text-sm text-content-primary">
          <span className="inline-flex items-center gap-1">
            <span className="font-mono">{workosTeam.workosTeamName}</span>
            <InfoTooltip
              items={[
                { label: "WorkOS Team ID", value: workosTeam.workosTeamId },
                { label: "Admin Email", value: workosTeam.workosAdminEmail },
              ]}
            />
          </span>{" "}
          <a
            href={`https://dashboard.workos.com/${workosTeam.workosTeamId}`}
            target="_blank"
            rel="noopener noreferrer"
            className="text-content-link hover:underline"
          >
            View in WorkOS
          </a>
        </p>

        <div className="flex flex-col gap-1 text-xs">
          {teamHealthData && (
            <>
              {teamInfo ? (
                <span className="inline-flex items-center gap-1 text-content-secondary">
                  <CheckIcon className="h-4 w-4" /> Convex has access to this
                  WorkOS workspace{" "}
                </span>
              ) : teamHealthData.error?.code === "WorkOSAPIUnavailable" ? (
                <span className="inline-flex items-center gap-1 text-content-warning">
                  <ExclamationTriangleIcon className="h-4 w-4" /> WorkOS API
                  currently unavailable
                </span>
              ) : !teamHealthData.error && !teamHealthData.data ? (
                <span className="text-content-tertiary">
                  Checking accessibility...
                </span>
              ) : (
                <span className="inline-flex items-center gap-1 text-content-warning">
                  <ExclamationTriangleIcon className="h-4 w-4" /> Unable to
                  verify workspace
                </span>
              )}
            </>
          )}
          {isLoadingHealth ? (
            <span className="text-content-tertiary">
              Checking payment status...
            </span>
          ) : teamInfo ? (
            canProvisionProduction ? (
              <span className="inline-flex items-center gap-1 text-content-secondary">
                <CheckIcon className="h-4 w-4" /> Payment method configured
              </span>
            ) : (
              <span className="inline-flex items-center gap-1 text-content-secondary">
                <ExclamationTriangleIcon className="h-4 w-4 flex-shrink-0 text-content-warning" />
                <span>
                  <a
                    href={`https://dashboard.workos.com/${workosTeam.workosTeamId}/settings/billing`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-content-link hover:underline"
                  >
                    Add payment method in WorkOS
                  </a>{" "}
                  to provision production AuthKit environments.
                </span>
              </span>
            )
          ) : null}
        </div>
      </div>

      <InviteTeamMemberSection teamId={teamId} />

      <ExpandableActionSection
        config={{
          trigger: {
            label: "Disconnect Workspace",
          },
          expanded: {
            title: "Disconnect WorkOS Workspace",
            description:
              "You can disassociate a WorkOS workspace from your team to revoke permission to create AuthKit environments in it from Convex. Existing WorkOS environments will continue to work.",
            variant: "danger",
            actions: {
              primary: {
                label: "Disconnect",
                onClick: handleDisconnect,
                variant: "danger",
              },
            },
          },
        }}
        isLoading={isDisconnecting}
      />
    </div>
  );
}

function ConsolidatedEnvironmentSection({
  environment,
  workosTeam,
  hasTeam,
  notSupportedReason,
  envVarsLink,
  workosEnvVars,
  deploymentName,
  deployment,
  canProvisionProduction,
}: {
  environment: ProvisionedEnvironment | null;
  workosTeam:
    | {
        workosTeamId: string;
        workosTeamName: string;
        workosAdminEmail: string;
      }
    | null
    | undefined;
  hasTeam: boolean;
  notSupportedReason: string | undefined;
  envVarsLink?: string;
  workosEnvVars: WorkOSEnvVars;
  deploymentName?: string;
  deployment: ReturnType<DeploymentInfo["useCurrentDeployment"]>;
  canProvisionProduction: boolean;
}) {
  const { workOSOperations } = useContext(DeploymentInfoContext);
  const updateEnvironmentVariables = useUpdateEnvVars();

  // Only check health if an environment is actually provisioned
  const envHealthData = workOSOperations.useWorkOSEnvironmentHealth(
    environment ? deploymentName : undefined,
  );
  const [provisioningType, setProvisioningType] = useState<
    "production" | "non-production" | null
  >(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [isSettingEnvVars, setIsSettingEnvVars] = useState(false);
  // Create mutations with the current deploymentName
  const provisionMutation =
    workOSOperations.useProvisionWorkOSEnvironment(deploymentName);
  const deleteMutation =
    workOSOperations.useDeleteWorkOSEnvironment(deploymentName);

  const handleProvisionEnvironment = async (isProduction: boolean) => {
    if (!deploymentName) return;

    setProvisioningType(isProduction ? "production" : "non-production");
    try {
      const result = await provisionMutation({
        deploymentName,
        isProduction,
      });

      // If we got an API key from provisioning, set it immediately
      if (result?.apiKey && result?.newlyProvisioned) {
        const changes = [
          { name: "WORKOS_CLIENT_ID", value: result.clientId },
          { name: "WORKOS_ENVIRONMENT_ID", value: result.environmentId },
          { name: "WORKOS_API_KEY", value: result.apiKey },
        ];

        try {
          await updateEnvironmentVariables(changes);
          toast(
            "success",
            "WorkOS environment provisioned and variables configured",
          );
        } catch (error) {
          console.error("Failed to set environment variables:", error);
          toast(
            "info",
            "Environment provisioned but variables need manual configuration",
          );
        }
      }

      // Success! The mutation will automatically revalidate the environment query
      setProvisioningType(null);
      setShowCreateForm(false);
    } catch (error) {
      const { message } = getErrorInfo(error);
      toast(
        "error",
        message || "Failed to provision environment. Please try again.",
      );
      setProvisioningType(null);
    }
  };

  const handleDeleteEnvironment = async () => {
    if (!deploymentName || !environment) return;

    setIsDeleting(true);
    try {
      await deleteMutation({ deploymentName });

      // Clear env vars that matched the deleted environment
      const envVarsToClear: { name: string; value: null }[] = [];
      if (workosEnvVars.clientId === environment.workosClientId) {
        envVarsToClear.push({ name: "WORKOS_CLIENT_ID", value: null });
      }
      if (workosEnvVars.environmentId === environment.workosEnvironmentId) {
        envVarsToClear.push({ name: "WORKOS_ENVIRONMENT_ID", value: null });
      }
      if (
        environment.workosApiKey &&
        workosEnvVars.apiKey === environment.workosApiKey
      ) {
        envVarsToClear.push({ name: "WORKOS_API_KEY", value: null });
      }

      if (envVarsToClear.length > 0) {
        try {
          await updateEnvironmentVariables(envVarsToClear);
        } catch (error) {
          console.error("Failed to clear environment variables:", error);
        }
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error
          ? error.message
          : "Failed to delete environment. Please try again.";
      toast("error", errorMessage);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleSetEnvironmentVariables = async () => {
    if (!environment) return;

    setIsSettingEnvVars(true);
    try {
      const changes = [
        { name: "WORKOS_CLIENT_ID", value: environment.workosClientId },
        {
          name: "WORKOS_ENVIRONMENT_ID",
          value: environment.workosEnvironmentId,
        },
        ...(environment.workosApiKey
          ? [{ name: "WORKOS_API_KEY", value: environment.workosApiKey }]
          : []),
      ];

      await updateEnvironmentVariables(changes);
      toast("success", "Environment variables have been set successfully");
    } catch (error) {
      const errorMessage =
        error instanceof Error
          ? error.message
          : "Failed to set environment variables. Please try again.";
      toast("error", errorMessage);
    } finally {
      setIsSettingEnvVars(false);
    }
  };

  const isProductionDeployment = deployment?.deploymentType === "prod";

  // Check if environment was created for a different team
  const teamMismatch =
    environment &&
    workosTeam &&
    environment.workosTeamId !== workosTeam.workosTeamId;

  // Check environment variable states
  const clientIdMatches =
    environment &&
    workosEnvVars.clientId &&
    workosEnvVars.clientId === environment.workosClientId;
  const environmentIdMatches =
    environment &&
    workosEnvVars.environmentId &&
    workosEnvVars.environmentId === environment.workosEnvironmentId;
  const apiKeyMatches =
    environment &&
    environment.workosApiKey &&
    workosEnvVars.apiKey &&
    workosEnvVars.apiKey === environment.workosApiKey;

  const clientIdMissing = environment && !workosEnvVars.clientId;
  const environmentIdMissing = environment && !workosEnvVars.environmentId;
  const apiKeyMissing = environment && !workosEnvVars.apiKey;

  const clientIdMismatch =
    environment && workosEnvVars.clientId && !clientIdMatches;
  const environmentIdMismatch =
    environment && workosEnvVars.environmentId && !environmentIdMatches;
  const apiKeyMismatch = environment && workosEnvVars.apiKey && !apiKeyMatches;

  return (
    <div className="flex flex-col gap-2">
      {environment ? (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-content-primary">
            <span className="inline-flex items-center gap-1">
              <span>{environment.workosEnvironmentName}</span>
              <InfoTooltip
                items={[
                  {
                    label: "WorkOS Environment ID",
                    value: environment.workosEnvironmentId,
                  },
                  {
                    label: "WorkOS Client ID",
                    value: environment.workosClientId,
                  },
                  ...(environment.workosApiKey
                    ? [
                        {
                          label: "WorkOS API Key",
                          value: environment.workosApiKey,
                          isSecret: true,
                        },
                      ]
                    : []),
                  {
                    label: "Environment Type",
                    value: environment.isProduction
                      ? "Production"
                      : "Non-Production",
                  },
                ]}
              />
            </span>{" "}
            <a
              href={`https://dashboard.workos.com/${environment.workosEnvironmentId}/authentication`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-content-link hover:underline"
            >
              View in WorkOS
            </a>
          </p>

          <div className="flex flex-col gap-1 text-xs">
            {envHealthData && (
              <>
                {envHealthData.data ? (
                  <span className="inline-flex items-center gap-1 text-content-secondary">
                    <CheckIcon className="h-4 w-4" /> Convex has access to this
                    environment
                  </span>
                ) : envHealthData.error?.code ===
                  "WorkOSEnvironmentNotFound" ? (
                  <span className="inline-flex items-center gap-1 text-content-error">
                    <Cross2Icon className="h-4 w-4" /> Environment not found in
                    WorkOS (may have been deleted)
                  </span>
                ) : envHealthData.error?.code === "WorkOSAPIUnavailable" ? (
                  <span className="inline-flex items-center gap-1 text-content-warning">
                    <ExclamationTriangleIcon className="h-4 w-4" /> WorkOS API
                    currently unavailable
                  </span>
                ) : !envHealthData.error ? (
                  <span className="text-content-tertiary">
                    Checking accessibility...
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-content-warning">
                    <ExclamationTriangleIcon className="h-4 w-4" /> Unable to
                    verify environment
                  </span>
                )}
              </>
            )}
            {teamMismatch ? (
              <span className="inline-flex items-center gap-1 text-content-warning">
                <ExclamationTriangleIcon className="h-4 w-4" /> Created for a
                different workspace than{" "}
                <span className="font-mono">{workosTeam?.workosTeamName}</span>
              </span>
            ) : workosTeam ? (
              <span className="inline-flex items-center gap-1 text-content-secondary">
                <CheckIcon className="h-4 w-4" /> Created by WorkOS workspace
                linked to this Convex team,{" "}
                <span className="font-mono">{workosTeam.workosTeamName}</span>
              </span>
            ) : null}
            {(() => {
              const changes: EnvVarChange[] = [];

              if (clientIdMissing || clientIdMismatch) {
                changes.push({
                  name: "WORKOS_CLIENT_ID",
                  currentValue: workosEnvVars.clientId,
                  newValue: environment.workosClientId,
                });
              }

              if (environmentIdMissing || environmentIdMismatch) {
                changes.push({
                  name: "WORKOS_ENVIRONMENT_ID",
                  currentValue: workosEnvVars.environmentId,
                  newValue: environment.workosEnvironmentId,
                });
              }

              if (
                environment.workosApiKey &&
                (apiKeyMissing || apiKeyMismatch)
              ) {
                changes.push({
                  name: "WORKOS_API_KEY",
                  currentValue: workosEnvVars.apiKey,
                  newValue: environment.workosApiKey,
                });
              }

              if (changes.length === 0) {
                return (
                  <span className="inline-flex items-center gap-1 text-content-secondary">
                    <CheckIcon className="h-4 w-4" />
                    <code>WORKOS_*</code> environment variables match the values
                    for the provisioned environment
                  </span>
                );
              }

              // Some variables need updating
              return (
                <div className="flex flex-col gap-2">
                  <span className="inline-flex items-center gap-1 text-content-warning">
                    <ExclamationTriangleIcon className="h-4 w-4" />
                    Environment variables need to be configured
                  </span>

                  <Callout variant="instructions">
                    <div className="flex w-full flex-col gap-3">
                      <div>
                        <p className="mb-2 text-sm font-semibold">
                          Update Deployment Environment Variables
                        </p>
                        <p className="text-xs text-content-secondary">
                          The following environment variables can be updated to
                          match the Convex-provisioned WorkOS environment for
                          this deployment:
                        </p>
                      </div>

                      <div className="flex flex-col gap-2">
                        {changes.map((change) => (
                          <EnvVarChangeRow key={change.name} change={change} />
                        ))}
                      </div>

                      <div className="flex gap-2">
                        <Button
                          size="sm"
                          onClick={handleSetEnvironmentVariables}
                          loading={isSettingEnvVars}
                        >
                          {changes.some((c) => c.currentValue !== null)
                            ? "Update Environment Variables"
                            : "Set Environment Variables"}
                        </Button>
                        {envVarsLink && (
                          <Button
                            size="sm"
                            variant="neutral"
                            href={envVarsLink}
                            target="_blank"
                          >
                            Configure Manually
                            <ExternalLinkIcon className="ml-1 h-3 w-3" />
                          </Button>
                        )}
                      </div>
                    </div>
                  </Callout>
                </div>
              );
            })()}
          </div>

          {environment &&
            !environment.isProduction &&
            (() => {
              const matchingVars = [
                clientIdMatches && "WORKOS_CLIENT_ID",
                environmentIdMatches && "WORKOS_ENVIRONMENT_ID",
                apiKeyMatches && "WORKOS_API_KEY",
              ].filter(Boolean) as string[];

              return (
                <ExpandableActionSection
                  config={{
                    trigger: {
                      label: "Delete Provisioned Environment",
                    },
                    expanded: {
                      title: (
                        <>
                          Delete WorkOS AuthKit environment{" "}
                          <span className="font-mono">
                            {environment.workosEnvironmentName}
                          </span>
                        </>
                      ),
                      description: (
                        <>
                          This will permanently delete this environment from
                          WorkOS and remove it from Convex. This action cannot
                          be undone.
                          {matchingVars.length > 0 && (
                            <>
                              {" "}
                              The following environment variables will also be
                              cleared:{" "}
                              <span className="font-mono">
                                {matchingVars.join(", ")}
                              </span>
                              .
                            </>
                          )}
                        </>
                      ),
                      variant: "danger",
                      actions: {
                        primary: {
                          label: "Delete",
                          onClick: handleDeleteEnvironment,
                          variant: "danger",
                        },
                      },
                    },
                  }}
                  isLoading={isDeleting}
                />
              );
            })()}
        </div>
      ) : workosEnvVars.clientId ? (
        <p className="text-sm text-content-secondary">
          Manually configured (<code className="text-xs">WORKOS_CLIENT_ID</code>{" "}
          is set)
        </p>
      ) : (
        <p className="text-sm text-content-secondary">
          No environment provisioned
        </p>
      )}

      {/* Not Supported Notice */}
      {notSupportedReason && (
        <Callout variant="instructions">
          <div className="flex flex-col gap-2">
            <p className="text-sm">{notSupportedReason}</p>
            <p className="text-sm">
              You can{" "}
              <Link
                href="https://docs.convex.dev/auth/authkit"
                className="text-content-link hover:underline"
                target="_blank"
              >
                configure WorkOS AuthKit manually
              </Link>{" "}
              by setting{" "}
              <Link
                href={`${envVarsLink}?var=WORKOS_CLIENT_ID&var=WORKOS_API_KEY`}
                className="text-content-link hover:underline"
              >
                environment variables
              </Link>
              .
            </p>
          </div>
        </Callout>
      )}

      {/* Actions - only show if not already provisioned and supported */}
      {!environment &&
        !notSupportedReason &&
        hasTeam &&
        (!showCreateForm ? (
          <div>
            <Button
              variant="neutral"
              size="sm"
              onClick={() => setShowCreateForm(true)}
            >
              Create Environment
            </Button>
          </div>
        ) : (
          <div className="flex flex-col gap-2 rounded-sm border p-3">
            <div className="flex flex-col gap-1">
              <p className="text-sm font-semibold text-content-primary">
                Create a new WorkOS environment for this deployment
              </p>
              <p className="text-xs text-content-secondary">
                {isProductionDeployment ? (
                  <>
                    WorkOS AuthKit environments are generally created by the
                    Convex CLI at deployment creation time but you can manually
                    create a{" "}
                    <a
                      href="https://workos.com/docs/authkit/modeling-your-app/single-tenant-and-multi-tenant-models/multi-tenant"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-content-link hover:underline"
                    >
                      production or staging environment
                    </a>{" "}
                    here. This will also configure the{" "}
                    <code className="text-xs">WORKOS_*</code> environment
                    variables for this deployment.
                  </>
                ) : (
                  <>
                    WorkOS AuthKit environments are generally created by the
                    Convex CLI at deployment creation time but you can manually
                    create one here. This will also configure the{" "}
                    <code className="text-xs">WORKOS_*</code> environment
                    variables for this deployment.
                  </>
                )}
              </p>
            </div>
            <div className="flex gap-2">
              {isProductionDeployment ? (
                <>
                  <Button
                    size="sm"
                    onClick={() => handleProvisionEnvironment(true)}
                    disabled={!canProvisionProduction}
                    loading={provisioningType === "production"}
                    tip={
                      canProvisionProduction
                        ? undefined
                        : "Add a payment method in WorkOS to create production environments"
                    }
                  >
                    Create Production Environment
                  </Button>
                  <Button
                    size="sm"
                    variant="neutral"
                    onClick={() => handleProvisionEnvironment(false)}
                    loading={provisioningType === "non-production"}
                  >
                    Create Staging Environment
                  </Button>
                </>
              ) : (
                <Button
                  size="sm"
                  onClick={() => handleProvisionEnvironment(false)}
                  loading={provisioningType === "non-production"}
                >
                  Create Environment
                </Button>
              )}
              <Button
                variant="neutral"
                size="sm"
                onClick={() => setShowCreateForm(false)}
                disabled={provisioningType !== null}
              >
                Cancel
              </Button>
            </div>
          </div>
        ))}
    </div>
  );
}

function ModalFooter() {
  return (
    <p className="text-sm text-content-primary">
      <Link
        href="https://docs.convex.dev/auth/authkit/auto-provision"
        className="text-content-link hover:underline"
        target="_blank"
      >
        Learn more about automatic creation of WorkOS environments
      </Link>
    </p>
  );
}

export function WorkOSConfigurationForm() {
  const [showTeamCreatedSuccess, setShowTeamCreatedSuccess] = useState(false);

  const handleTeamCreated = () => {
    setShowTeamCreatedSuccess(true);
  };

  const {
    useCurrentDeployment,
    useCurrentTeam,
    useCurrentProject,
    workOSOperations,
  } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const workosData = workOSOperations.useDeploymentWorkOSEnvironment(
    deployment?.name,
  );
  const teamHealthData = workOSOperations.useWorkOSTeamHealth(
    team?.id?.toString(),
  );
  const notSupportedReason = getWorkOSNotSupportedReason(deployment);

  const workosEnvVars = useRelevantEnvVars();

  const envVarsLink =
    team && project && deployment
      ? `/t/${team.slug}/${project.slug}/${deployment.name}/settings/environment-variables`
      : undefined;

  if (!workosData || !workosEnvVars) {
    return (
      <div className="flex h-32 items-center justify-center">
        <Loading />
      </div>
    );
  }

  const { workosTeam, environment } = workosData;
  const hasTeam = workosTeam !== null && workosTeam !== undefined;
  const teamInfo = teamHealthData?.data?.teamProvisioned
    ? teamHealthData.data.teamInfo
    : null;
  const canProvisionProduction = teamInfo?.productionState === "active";

  // Hide the environment section when there's no team, no environment, and no
  // manually configured WORKOS_CLIENT_ID - focus on creating a workspace first
  const showEnvironmentSection =
    hasTeam || environment !== null || workosEnvVars.clientId !== null;

  return (
    <div className="flex flex-col gap-6">
      {showEnvironmentSection && (
        <>
          <div className="flex flex-col gap-2">
            <div className="text-sm font-semibold text-content-primary">
              AuthKit Environment for this Deployment
            </div>

            <ConsolidatedEnvironmentSection
              environment={environment ?? null}
              workosTeam={workosTeam}
              hasTeam={hasTeam}
              notSupportedReason={notSupportedReason}
              envVarsLink={envVarsLink}
              workosEnvVars={workosEnvVars}
              deploymentName={deployment?.name}
              deployment={deployment}
              canProvisionProduction={canProvisionProduction}
            />
          </div>

          <div className="border-t" />
        </>
      )}

      <div className="flex flex-col gap-2">
        <div className="text-sm font-semibold text-content-primary">
          WorkOS Workspace for Team
        </div>

        <WorkOSTeamSection
          workosTeam={workosTeam}
          environment={environment ?? null}
          teamId={team?.id}
          showCongratulations={showTeamCreatedSuccess}
          onTeamCreated={handleTeamCreated}
        />
      </div>

      <ModalFooter />
    </div>
  );
}
