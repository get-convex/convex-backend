import { Button, Tooltip, Sheet, useCopy } from "dashboard-common";
import { CopyIcon } from "@radix-ui/react-icons";
import { TextInput } from "elements/TextInput";
import { useFormik } from "formik";
import { useUpdateProject } from "api/projects";
import { ProjectDetails, Team } from "generatedApi";
import * as Yup from "yup";

const TeamSchema = Yup.object().shape({
  name: Yup.string()
    .min(3, "Project name must be at least 3 characters long.")
    .max(128, "Project name must be at most 128 characters long.")
    .required("Project name is required."),
  slug: Yup.string()
    .min(3, "Project slug must be at least 3 characters long.")
    .max(64, "Project slug must be at most 64 characters long.")
    .matches(
      /^[\w-]+$/,
      "Project slug may contain numbers, letters, underscores, and '-'.",
    )
    .required(),
});

export function ProjectForm({
  project,
  team,
  hasAdminPermissions,
}: {
  team: Team;
  project: ProjectDetails;
  hasAdminPermissions: boolean;
}) {
  const updateProject = useUpdateProject(project.id);
  const formState = useFormik({
    initialValues: {
      name: project.name,
      slug: project.slug,
    },
    enableReinitialize: true,
    validationSchema: TeamSchema,
    onSubmit: async (values) => {
      await updateProject(values);
      // Completely reload the page to avoid race conditions
      // with the slug of the project being updated.
      window.location.href = `/t/${team.slug}/${values.slug}/settings`;
    },
  });

  const copyToClipboard = useCopy("Project slug");

  return (
    <Sheet className="text-sm">
      <h3 className="mb-4">Edit Project</h3>
      <form
        onSubmit={formState.handleSubmit}
        aria-label="Edit project settings"
      >
        <div className="mb-6 flex max-w-xs flex-col gap-4">
          <Tooltip
            tip={
              !hasAdminPermissions
                ? "You do not have permission to update the project name."
                : undefined
            }
          >
            <TextInput
              label="Project Name"
              outerClassname="max-w-[20rem]"
              placeholder="Enter a name for your project"
              onChange={formState.handleChange}
              value={formState.values.name}
              id="name"
              error={formState.errors.name}
              disabled={!hasAdminPermissions}
            />
          </Tooltip>

          <Tooltip
            tip={
              !hasAdminPermissions
                ? "You do not have permission to update the project slug."
                : undefined
            }
          >
            <TextInput
              label="Project Slug"
              outerClassname="max-w-[20rem]"
              placeholder="Enter a slug for your project"
              onChange={formState.handleChange}
              value={formState.values.slug}
              Icon={CopyIcon}
              action={() => copyToClipboard(formState.values.slug)}
              id="slug"
              error={formState.errors.slug}
              disabled={!hasAdminPermissions}
            />
          </Tooltip>
        </div>

        <Button
          className="float-right"
          disabled={
            !formState.dirty || formState.isSubmitting || !formState.isValid
          }
          type="submit"
          aria-label="submit"
        >
          Save
        </Button>
      </form>
    </Sheet>
  );
}
