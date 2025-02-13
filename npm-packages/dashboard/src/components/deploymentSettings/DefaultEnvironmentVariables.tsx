import { useState } from "react";
import { Sheet } from "dashboard-common/elements/Sheet";
import {
  useProjectEnvironmentVariables,
  useUpdateProjectEnvVars,
} from "hooks/api";
import { useCurrentProject } from "api/projects";
import { useHasProjectAdminPermissions } from "api/roles";
import { DeploymentType as DeploymentTypeType } from "generatedApi";
import Link from "next/link";
import { EnvironmentVariables } from "dashboard-common/features/settings/components/EnvironmentVariables";
import { DeploymentType } from "dashboard-common/features/settings/components/DeploymentUrl";

const DEPLOYMENT_TYPES_FOR_DEFAULT_ENV_VARIABLES: DeploymentTypeType[] = [
  "dev",
  "preview",
];

export function DefaultEnvironmentVariables() {
  const project = useCurrentProject();
  const projectId = project?.id;
  const environmentVariables = useProjectEnvironmentVariables(
    projectId,
    100,
  )?.configs;
  const updateEnvironmentVariables = useUpdateProjectEnvVars(projectId);

  const [initialValues, setInitialValues] = useState(undefined);

  const hasAdminPermissions = useHasProjectAdminPermissions(projectId);

  return (
    <Sheet className="flex flex-col gap-4 text-sm">
      <h3>Default Environment Variables</h3>
      <div className="flex flex-col gap-2">
        <p className="max-w-prose text-sm text-content-primary">
          These values will be used when creating new{" "}
          <DeploymentType deploymentType="dev" /> and{" "}
          <DeploymentType deploymentType="preview" /> deployments. Changing
          these values <span className="font-semibold">does not</span> affect
          existing deployments.{" "}
          <Link
            passHref
            href="https://docs.convex.dev/production/hosting/environment-variables#project-environment-variable-defaults"
            className="text-content-link"
            target="_blank"
          >
            Learn more
          </Link>
        </p>
        <p className="max-w-prose text-sm text-content-primary">
          The environment variables for an existing deployment can be viewed and
          managed from the deployment settings.
        </p>
      </div>
      <EnvironmentVariables
        hasAdminPermissions={hasAdminPermissions}
        environmentVariables={environmentVariables}
        updateEnvironmentVariables={async (
          creations,
          modifications,
          deletions,
        ) => {
          await updateEnvironmentVariables({
            changes: [
              ...creations.map((newEnvVar) => ({
                oldVariable: null,
                newConfig: {
                  ...newEnvVar,
                  deploymentTypes: DEPLOYMENT_TYPES_FOR_DEFAULT_ENV_VARIABLES,
                },
              })),
              ...modifications.map(({ oldEnvVar, newEnvVar }) => ({
                oldVariable: oldEnvVar,
                newConfig: {
                  ...newEnvVar,
                  deploymentTypes: DEPLOYMENT_TYPES_FOR_DEFAULT_ENV_VARIABLES,
                },
              })),
              ...deletions.map((oldVariable) => ({
                oldVariable,
                newConfig: null,
              })),
            ],
          });
          setInitialValues(undefined);
        }}
        initialFormValues={initialValues}
      />
    </Sheet>
  );
}
