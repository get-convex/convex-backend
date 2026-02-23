import {
  useCurrentDeployment,
  useModifyDeploymentSettings,
} from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useCallback, useId, useState } from "react";
import { useFormik } from "formik";
import * as Yup from "yup";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Pencil1Icon } from "@radix-ui/react-icons";
import * as Sentry from "@sentry/nextjs";
import { Sheet } from "@ui/Sheet";
import { CopyButton } from "@common/elements/CopyButton";

const referenceValidationSchema = Yup.object().shape({
  reference: Yup.string()
    .required("Reference is required")
    .min(3, "Reference must be at least 3 characters")
    .max(100, "Reference must be at most 100 characters")
    .matches(
      /^[a-z0-9/-]+$/,
      "Reference can only contain lowercase letters, numbers, hyphens, and slashes",
    )
    .test(
      "not-deployment-name-format",
      "Reference cannot be in the format abc-xyz-123, as it is reserved for deployment names",
      (value) => {
        if (!value) return true;
        // Check if it matches the deployment name pattern: word-word-number
        return !/^[a-z]+-[a-z]+-\d+$/.test(value);
      },
    )
    .test(
      "not-local-prefix",
      "Reference cannot start with 'local-'",
      (value) => {
        if (!value) return true;
        return !value.startsWith("local-");
      },
    )
    .test(
      "not-reserved",
      // eslint-disable-next-line no-template-curly-in-string -- Yup error template
      '"${value}" is a reserved name and cannot be used as a reference.',
      (value) => {
        if (!value) return true;
        const reserved = [
          "prod",
          "dev",
          "cloud",
          "local",
          "default",
          "name",
          "new",
          "existing",
          "deployment",
          "preview",
        ];
        return !reserved.includes(value);
      },
    ),
});

export function DeploymentReference() {
  const deployment = useCurrentDeployment();

  const project = useCurrentProject();
  const modifyDeploymentSettings = useModifyDeploymentSettings({
    deploymentName: deployment?.name,
    projectId: project?.id,
  });

  const handleUpdateReference = useCallback(
    async (reference: string) => {
      await modifyDeploymentSettings({ reference });
    },
    [modifyDeploymentSettings],
  );

  // We hide the section when `deployment` is loading.
  // This is fine since for the cloud dashboard, `useCurrentDeployment` loads during SSR
  if (deployment === undefined) return null;

  // Local deployments have no references
  if (deployment.kind === "local") return null;

  return (
    <DeploymentReferenceInner
      value={deployment.reference}
      onUpdate={handleUpdateReference}
    />
  );
}

export function DeploymentReferenceInner({
  value,
  onUpdate,
}: {
  value: string;
  onUpdate: (reference: string) => Promise<void>;
}) {
  const textFieldId = useId();

  const [isEditing, setIsEditing] = useState(false);

  const formState = useFormik({
    initialValues: {
      reference: value,
    },
    validationSchema: referenceValidationSchema,
    onSubmit: async (values) => {
      if (values.reference === undefined) {
        Sentry.captureMessage(
          "Unexpectedly submitting DeploymentReferenceInner with an undefined value",
          "error",
        );
        return;
      }

      await onUpdate(values.reference);
      setIsEditing(false);
    },
    enableReinitialize: true,
  });

  const handleCancel = useCallback(() => {
    formState.resetForm();
    setIsEditing(false);
  }, [formState]);

  return (
    <Sheet>
      <div className="text-content-primary">
        <div className="mb-4 flex items-center justify-between">
          <h4>Deployment Reference</h4>
        </div>
        <p className="mb-2 text-sm">
          You can use the reference to target this deployment from the CLI{" "}
          <br />
          (e.g. <code>--deployment&nbsp;{value ?? "<reference>"}</code>).
        </p>

        <div className="flex flex-wrap items-start gap-x-2 gap-y-4 sm:flex-nowrap">
          {!isEditing ? (
            <>
              <TextInput
                id={textFieldId}
                label="Reference"
                labelHidden
                value={value}
                disabled
              />
              <CopyButton
                text={value ?? ""}
                disabled={value === undefined}
                size="sm"
              />
              <Button
                variant="neutral"
                onClick={() => setIsEditing(true)}
                icon={<Pencil1Icon />}
                aria-label="Edit deployment reference"
              >
                Edit
              </Button>
            </>
          ) : (
            <form onSubmit={formState.handleSubmit} className="contents">
              <TextInput
                id={textFieldId}
                label="Reference"
                labelHidden
                error={
                  (formState.touched.reference && formState.errors.reference) ||
                  undefined
                }
                disabled={formState.isSubmitting}
                {...formState.getFieldProps("reference")}
              />
              <Button
                type="button"
                variant="neutral"
                onClick={handleCancel}
                disabled={formState.isSubmitting}
              >
                Undo Edit
              </Button>
              <Button
                type="submit"
                variant="primary"
                disabled={formState.isSubmitting || !formState.isValid}
                loading={formState.isSubmitting}
              >
                Save
              </Button>
            </form>
          )}
        </div>
      </div>
    </Sheet>
  );
}
