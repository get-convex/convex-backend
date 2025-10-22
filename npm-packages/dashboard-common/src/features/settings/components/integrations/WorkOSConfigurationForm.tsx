import { useContext, useMemo } from "react";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import {
  DeploymentInfo,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import Link from "next/link";
import { Loading } from "@ui/Loading";
import { Tooltip } from "@ui/Tooltip";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { EnvironmentVariable } from "system-udfs/convex/_system/frontend/common";
import { Callout } from "@ui/Callout";

type WorkOSEnvVars = {
  clientId: string | null;
  environmentId: string | null;
  apiKey: string | null;
};

type ProvisionedEnvironment = {
  workosEnvironmentId: string;
  workosEnvironmentName: string;
  workosClientId: string;
};

function InfoTooltip({
  items,
}: {
  items: Array<{ label: string; value: string }>;
}) {
  return (
    <Tooltip
      maxWidthClassName="max-w-md"
      tip={
        <div className="flex flex-col gap-3 p-1 text-left">
          {items.map((item) => (
            <div key={item.label}>
              <div className="mb-1.5 text-xs font-semibold">{item.label}</div>
              <div className="max-w-full overflow-hidden">
                <CopyTextButton
                  text={item.value}
                  className="w-full font-mono text-xs [&_span]:break-all [&_span]:whitespace-normal"
                />
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

function getWorkOSNotSupportedReason(
  deployment: ReturnType<DeploymentInfo["useCurrentDeployment"]>,
): string | undefined {
  if (!deployment) {
    return undefined;
  }
  const { deploymentType, kind } = deployment;
  if (deploymentType === "dev" && kind === "cloud") {
    return undefined;
  }
  const labels = {
    dev: "local",
    prod: "production",
    preview: "preview",
  } as const;

  return `Automatic environment creation for ${labels[deploymentType]} deployments is not supported.`;
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
      (envVar) => envVar.name === "WORKOS_ENVIRONMENT_API_KEY",
    )?.value;

    return {
      clientId: clientId || null,
      environmentId: environmentId || null,
      apiKey: apiKey || null,
    };
  }, [environmentVariables]);

  return workosEnvVars;
}

function WorkOSTeamSection({
  workosTeam,
  environment: _environment,
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
  } | null;
}) {
  if (!workosTeam) {
    return (
      <p className="text-sm text-content-primary">
        This team does not have a WorkOS workspace. Create a new WorkOS
        workspace by{" "}
        <Link
          href="https://docs.convex.dev/auth/authkit/#get-started"
          className="text-content-link hover:underline"
          target="_blank"
        >
          deploying a template that uses AuthKit
        </Link>
        .
      </p>
    );
  }

  return (
    <p className="text-sm text-content-primary">
      The WorkOS workspace{" "}
      <span className="inline-flex items-center gap-1">
        <span className="font-mono">{workosTeam.workosTeamName}</span>
        <InfoTooltip
          items={[
            { label: "WorkOS Team ID", value: workosTeam.workosTeamId },
            { label: "Admin Email", value: workosTeam.workosAdminEmail },
          ]}
        />
      </span>{" "}
      linked to your team was created by Convex.
    </p>
  );
}

function ProvisionedEnvironmentSection({
  environment,
  hasTeam,
  notSupportedReason,
  envVarsLink,
  workosEnvVars,
}: {
  environment: ProvisionedEnvironment | null;
  hasTeam: boolean;
  notSupportedReason: string | undefined;
  envVarsLink?: string;
  workosEnvVars: WorkOSEnvVars;
}) {
  if (environment) {
    return (
      <p className="text-sm text-content-primary">
        The WorkOS environment{" "}
        <span className="inline-flex items-center gap-1">
          <span className="font-mono">{environment.workosEnvironmentName}</span>
          <InfoTooltip
            items={[
              {
                label: "WorkOS Environment ID",
                value: environment.workosEnvironmentId,
              },
              { label: "WorkOS Client ID", value: environment.workosClientId },
            ]}
          />
        </span>{" "}
        was created by Convex for this deployment.{" "}
        <a
          href={`https://dashboard.workos.com/${environment.workosEnvironmentId}/authentication`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-content-link hover:underline"
        >
          Go to WorkOS
        </a>
      </p>
    );
  }

  if (!hasTeam && !workosEnvVars.clientId) {
    return null;
  }

  // No environment and not supported
  if (notSupportedReason) {
    return (
      <Callout variant="instructions">
        <div className="flex flex-col gap-2">
          <p>{notSupportedReason}</p>
          <p>
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
              href={`${envVarsLink}?var=WORKOS_CLIENT_ID&var=WORKOS_ENVIRONMENT_API_KEY`}
              className="text-content-link hover:underline"
            >
              environment variables
            </Link>
            .
          </p>
        </div>
      </Callout>
    );
  }

  // No environment provisioned
  return (
    <>
      <p className="text-sm text-content-primary">
        {workosEnvVars.clientId ? (
          hasTeam ? (
            <>
              This deployment's{" "}
              <code className="text-xs">WORKOS_CLIENT_ID</code> environment
              variable is already set to a WorkOS environment. If you've like to
              instead use a new WorkOS environment created just for this
              deployment,{" "}
              <Link
                href={`${envVarsLink}`}
                className="text-content-link hover:underline"
              >
                clear this environment variable
              </Link>{" "}
              and run this command:
            </>
          ) : (
            <>
              This deployment's{" "}
              <code className="text-xs">WORKOS_CLIENT_ID</code> environment
              variable is already set to a WorkOS environment.
            </>
          )
        ) : (
          "No WorkOS environment has been created for this deployment."
        )}
      </p>
      {hasTeam && (
        <CopyTextButton
          text="npx convex integration workos provision-environment"
          className="font-mono text-xs"
        />
      )}
    </>
  );
}

function EnvironmentVariablesWarnings({
  workosEnvVars,
  environment,
  envVarsLink,
}: {
  workosEnvVars: WorkOSEnvVars;
  environment: ProvisionedEnvironment | null;
  envVarsLink?: string;
}) {
  if (!environment) {
    return null;
  }

  const clientIdMatches =
    workosEnvVars.clientId &&
    workosEnvVars.clientId === environment.workosClientId;

  const environmentIdMatches =
    workosEnvVars.environmentId &&
    workosEnvVars.environmentId === environment.workosEnvironmentId;

  const clientIdMissing = !workosEnvVars.clientId;
  const environmentIdMissing = !workosEnvVars.environmentId;
  const clientIdMismatch = workosEnvVars.clientId && !clientIdMatches;
  const environmentIdMismatch =
    workosEnvVars.environmentId && !environmentIdMatches;

  const hasWarnings =
    clientIdMissing ||
    environmentIdMissing ||
    clientIdMismatch ||
    environmentIdMismatch;

  if (!hasWarnings) {
    return null;
  }

  const issues: string[] = [];
  if (clientIdMissing) issues.push("WORKOS_CLIENT_ID is not set");
  if (environmentIdMissing) issues.push("WORKOS_ENVIRONMENT_ID is not set");
  if (clientIdMismatch) issues.push("WORKOS_CLIENT_ID doesn't match");
  if (environmentIdMismatch) issues.push("WORKOS_ENVIRONMENT_ID doesn't match");

  return (
    <Callout variant="instructions">
      <div className="flex w-full flex-col gap-3">
        <p>
          {issues.length === 1 ? (
            <>
              The <code className="text-xs">{issues[0].split(" ")[0]}</code>{" "}
              environment variable{" "}
              {issues[0].includes("not set")
                ? "is not set"
                : "doesn't match the provisioned environment"}
              .
            </>
          ) : (
            <>
              Environment variables need to be updated:
              <ul className="mt-1 ml-4 list-disc">
                {issues.map((issue) => (
                  <li key={issue}>{issue}</li>
                ))}
              </ul>
            </>
          )}
        </p>

        <div className="flex flex-col gap-2">
          <div>
            <div className="mb-1 text-xs font-semibold">WORKOS_CLIENT_ID</div>
            <CopyTextButton
              text={environment.workosClientId}
              className="font-mono text-xs font-normal"
            />
          </div>
          <div>
            <div className="mb-1 text-xs font-semibold">
              WORKOS_ENVIRONMENT_ID
            </div>
            <CopyTextButton
              text={environment.workosEnvironmentId}
              className="font-mono text-xs font-normal"
            />
          </div>
        </div>

        {envVarsLink && (
          <Link
            href={envVarsLink}
            className="text-content-link hover:underline"
          >
            Go to environment variables
          </Link>
        )}
      </div>
    </Callout>
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
  const {
    useCurrentDeployment,
    useDeploymentWorkOSEnvironment,
    useCurrentTeam,
    useCurrentProject,
  } = useContext(DeploymentInfoContext);

  const deployment = useCurrentDeployment();
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const workosData = useDeploymentWorkOSEnvironment(deployment?.name);
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

  return (
    <div className="flex flex-col gap-2">
      <ProvisionedEnvironmentSection
        environment={environment ?? null}
        hasTeam={hasTeam}
        notSupportedReason={notSupportedReason}
        envVarsLink={envVarsLink}
        workosEnvVars={workosEnvVars}
      />

      <EnvironmentVariablesWarnings
        workosEnvVars={workosEnvVars}
        environment={environment ?? null}
        envVarsLink={envVarsLink}
      />

      <WorkOSTeamSection
        workosTeam={workosTeam}
        environment={environment ?? null}
      />

      <ModalFooter />
    </div>
  );
}
