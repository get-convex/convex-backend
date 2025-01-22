import { CopyIcon } from "@radix-ui/react-icons";
import { Button, Tooltip, Sheet, useCopy } from "dashboard-common";
import { TextInput } from "elements/TextInput";
import { useFormik } from "formik";
import { Team } from "generatedApi";
import * as Yup from "yup";

export type TeamFormProps = {
  team: Team;
  onUpdateTeam: (body: { name: string; slug: string }) => void;
  hasAdminPermissions: boolean;
};

const TeamSchema = Yup.object().shape({
  name: Yup.string()
    .min(3, "Team name must be at least 3 characters long.")
    .max(128, "Team name must be at most 128 characters long.")
    .required("Team name is required."),
  slug: Yup.string()
    .min(3, "Team slug must be at least 3 characters long.")
    .max(64, "Team slug must be at most 64 characters long.")
    .matches(
      /^[\w-]+$/,
      "Team slug may contain numbers, letters, underscores, and '-'.",
    )
    .required(),
});
export function TeamForm({
  team,
  onUpdateTeam,
  hasAdminPermissions,
}: TeamFormProps) {
  const formState = useFormik({
    initialValues: {
      name: team.name,
      slug: team.slug,
    },
    enableReinitialize: true,
    validationSchema: TeamSchema,
    onSubmit: async (values) => {
      await onUpdateTeam(values);
      // Completely reload the page to avoid race conditions
      // with the slug of the team being update.
      window.location.href = `/t/${values.slug}/settings`;
    },
  });

  const copyToClipboard = useCopy("Team slug");

  return (
    <Sheet className="text-sm">
      <h3 className="mb-4">Edit Team</h3>
      <form onSubmit={formState.handleSubmit} aria-label="Edit team settings">
        <div className="mb-6 flex max-w-xs flex-col gap-4">
          <Tooltip
            tip={
              !hasAdminPermissions
                ? "You do not have permission to update the team name."
                : undefined
            }
          >
            <TextInput
              label="Team Name"
              outerClassname="max-w-[20rem]"
              placeholder="Enter a name for your team"
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
                ? "You do not have permission to update the team slug."
                : undefined
            }
          >
            <TextInput
              label="Team Slug"
              outerClassname="max-w-[20rem]"
              placeholder="Enter a slug for your team"
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
            !formState.dirty ||
            formState.isSubmitting ||
            !formState.isValid ||
            !hasAdminPermissions
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
