import { useState } from "react";
import { useFormikContext } from "formik";
import { Sheet } from "@ui/Sheet";
import { Checkbox } from "@ui/Checkbox";
import {
  useProjectEnvironmentVariables,
  useUpdateProjectEnvVars,
} from "api/environmentVariables";
import { useCurrentProject } from "api/projects";
import { useHasProjectAdminPermissions } from "api/roles";
import {
  DeploymentType as DeploymentTypeType,
  UpdateEnvironmentVariables,
} from "generatedApi";
import Link from "next/link";
import { EnvironmentVariables } from "@common/features/settings/components/EnvironmentVariables";
import { ProjectEnvVarConfig } from "@common/features/settings/lib/types";

// Deployment types that can be selected via checkboxes (excludes "custom" for now)
const SELECTABLE_DEPLOYMENT_TYPES: DeploymentTypeType[] = [
  "dev",
  "preview",
  "prod",
];

const DEFAULT_DEPLOYMENT_TYPES: DeploymentTypeType[] = [
  ...SELECTABLE_DEPLOYMENT_TYPES,
];

export function DefaultEnvironmentVariables() {
  const project = useCurrentProject();
  const projectId = project?.id;
  const environmentVariables = useProjectEnvironmentVariables(
    projectId,
    100,
  )?.configs;
  const updateEnvironmentVariables = useUpdateProjectEnvVars(projectId);
  const hasAdminPermissions = useHasProjectAdminPermissions(projectId);

  return (
    <DefaultEnvironmentVariablesInner
      environmentVariables={environmentVariables}
      onUpdate={updateEnvironmentVariables}
      hasAdminPermissions={hasAdminPermissions}
    />
  );
}

export function DefaultEnvironmentVariablesInner({
  environmentVariables,
  onUpdate,
  hasAdminPermissions,
}: {
  environmentVariables: ProjectEnvVarConfig[] | undefined;
  onUpdate: (value: UpdateEnvironmentVariables) => Promise<void>;
  hasAdminPermissions: boolean;
}) {
  const [initialValues, setInitialValues] = useState<
    ProjectEnvVarConfig[] | undefined
  >(undefined);

  return (
    <Sheet className="flex flex-col gap-4 text-sm">
      <h3>Default Environment Variables</h3>
      <div className="flex flex-col gap-2">
        <p className="max-w-prose text-sm text-content-primary">
          These values will be used when creating new deployments. Changing
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
          await onUpdate({
            changes: [
              ...creations.map((newEnvVar) => ({
                oldVariable: null,
                newConfig: {
                  name: newEnvVar.name,
                  value: newEnvVar.value,
                  deploymentTypes: [...newEnvVar.deploymentTypes],
                },
              })),
              ...modifications.map(({ oldEnvVar, newEnvVar }) => ({
                oldVariable: oldEnvVar,
                newConfig: {
                  name: newEnvVar.name,
                  value: newEnvVar.value,
                  deploymentTypes: [...newEnvVar.deploymentTypes],
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
        renderDisplayExtra={DeploymentTypeLabels}
        renderEditExtra={DeploymentTypeCheckboxes}
        validateNameUniqueness={validateProjectEnvVarUniqueness}
        initEnvVar={(envVar) => ({
          ...envVar,
          deploymentTypes: DEFAULT_DEPLOYMENT_TYPES,
        })}
        envVarKey={envVarWithDtypesKey}
      />
    </Sheet>
  );
}

// Allows duplicate names if their deployment types don't overlap
export function validateProjectEnvVarUniqueness(
  allVariables: Array<{
    name: string;
    formKey: string;
    envVar: ProjectEnvVarConfig;
  }>,
): Record<string, string> {
  const errors: Record<string, string> = {};

  // Check for empty deployment types
  allVariables.forEach(({ formKey, envVar }) => {
    const selectableTypes = envVar.deploymentTypes.filter((t) =>
      SELECTABLE_DEPLOYMENT_TYPES.includes(t),
    );
    if (selectableTypes.length === 0) {
      errors[`${formKey}.deploymentTypes`] =
        "At least one deployment type must be selected";
    }
  });

  // Group by name
  const byName = new Map<
    string,
    Array<{ formKey: string; deploymentTypes: readonly DeploymentTypeType[] }>
  >();

  allVariables.forEach(({ name, formKey, envVar }) => {
    // We don’t need a conflict error messages for empty env var names
    // because there will be an error in the name field itself
    if (name === "") return;

    const existing = byName.get(name) || [];
    existing.push({ formKey, deploymentTypes: envVar.deploymentTypes });
    byName.set(name, existing);
  });

  // Check for deployment type overlaps within each name group
  byName.forEach((entries) => {
    if (entries.length <= 1) return;

    // Check all pairs for deployment type intersection
    for (let i = 0; i < entries.length; i++) {
      for (let j = i + 1; j < entries.length; j++) {
        const intersection = entries[i].deploymentTypes.filter((dt) =>
          entries[j].deploymentTypes.includes(dt),
        );
        if (intersection.length > 0) {
          // Mark both as having errors
          const conflictMsg = `Conflicts with another variable for: ${intersection.map(deploymentTypeName).join(", ")}`;
          errors[`${entries[i].formKey}.deploymentTypes`] = conflictMsg;
          errors[`${entries[j].formKey}.deploymentTypes`] = conflictMsg;
        }
      }
    }
  });

  return errors;
}

function DeploymentTypeLabels({ envVar }: { envVar: ProjectEnvVarConfig }) {
  return (
    <div className="mt-0.5 flex flex-wrap gap-1 text-xs text-content-tertiary">
      {envVar.deploymentTypes.map(deploymentTypeName).join(", ")}
    </div>
  );
}

function DeploymentTypeCheckboxes({
  formKey,
  envVar,
}: {
  formKey: string;
  envVar: ProjectEnvVarConfig;
}) {
  const formState = useFormikContext();
  const checkboxKey = `${formKey}.deploymentTypes`;
  const error = (formState.errors as Record<string, string>)[checkboxKey];
  const { deploymentTypes } = envVar;

  const handleToggle = (type: DeploymentTypeType) => {
    const currentTypes = deploymentTypes;
    const newTypes = currentTypes.includes(type)
      ? currentTypes.filter((t) => t !== type)
      : [...currentTypes, type];

    void formState.setFieldValue(checkboxKey, newTypes);
  };

  const legend = "Deployment types:";

  return (
    <div className="flex flex-col gap-2">
      <fieldset className="flex flex-wrap items-center gap-x-3 gap-y-2">
        {/* Duplicating the legend because browsers won’t let me style it */}
        <legend className="sr-only">{legend}</legend>
        <div aria-hidden className="text-content-tertiary">
          {legend}
        </div>

        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          {" "}
          {SELECTABLE_DEPLOYMENT_TYPES.map((type) => {
            const isChecked = deploymentTypes.includes(type);
            return (
              <label
                key={type}
                className="flex cursor-pointer items-center gap-1.5 text-sm text-content-primary"
              >
                <Checkbox
                  checked={isChecked}
                  onChange={() => handleToggle(type)}
                  disabled={formState.isSubmitting}
                  className={deploymentTypeCheckedBackground(type)}
                />
                {deploymentTypeName(type)}
              </label>
            );
          })}
        </div>
      </fieldset>

      {error && (
        <p
          className="flex max-w-full animate-fadeInFromLoading gap-1 text-xs break-words text-content-errorSecondary"
          role="alert"
        >
          {error}
        </p>
      )}
    </div>
  );
}

function deploymentTypeName(dtype: DeploymentTypeType) {
  switch (dtype) {
    case "prod":
      return "Production";
    case "preview":
      return "Preview";
    case "dev":
      return "Development";
    case "custom":
      return "Custom";
    default: {
      dtype satisfies never;
      return "Unknown";
    }
  }
}

function deploymentTypeCheckedBackground(dtype: DeploymentTypeType) {
  switch (dtype) {
    case "prod":
      return "checked:bg-purple-700 text-purple-700 enabled:hover:checked:bg-purple-800 dark:checked:bg-purple-500 dark:enabled:hover:bg-purple-500 dark:text-purple-500";
    case "preview":
      return "checked:bg-orange-700 text-orange-700 enabled:hover:checked:bg-orange-800 dark:checked:bg-orange-700 dark:enabled:hover:bg-orange-600 dark:text-orange-400";
    case "dev":
      return "checked:bg-green-700 text-green-700 enabled:hover:checked:bg-green-800 dark:checked:bg-green-700 dark:enabled:hover:bg-green-600 dark:text-green-400";
    case "custom":
      return "checked:bg-neutral-8 text-neutral-8 enabled:hover:checked:bg-neutral-9 dark:checked:bg-neutral-6 dark:enabled:hover:bg-neutral-5 dark:text-neutral-5";
    default: {
      dtype satisfies never;
      return "";
    }
  }
}

function envVarWithDtypesKey(envVar: ProjectEnvVarConfig) {
  return `${envVar.name} (${envVar.deploymentTypes.map(deploymentTypeName).join(" ")})`;
}
