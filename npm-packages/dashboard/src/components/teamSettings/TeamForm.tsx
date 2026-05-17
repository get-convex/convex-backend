import { CopyIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Sheet } from "@ui/Sheet";
import { TextInput } from "@ui/TextInput";
import { useFormik } from "formik";
import { RegionName, TeamResponse } from "generatedApi";
import * as Yup from "yup";
import { useCopy } from "@common/lib/useCopy";
import { useDeploymentRegions } from "api/deployments";
import { permissionDeniedTip } from "elements/permissionDeniedTip";
import { DefaultRegionSelector } from "./DefaultRegionSelector";

export type TeamFormProps = {
  team: TeamResponse;
  onUpdateTeam: (body: {
    name: string;
    slug: string;
    defaultRegion: RegionName | null;
  }) => Promise<void>;
  canUpdate: boolean;
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
  defaultRegion: Yup.string().nullable(),
});
export function TeamForm({ team, onUpdateTeam, canUpdate }: TeamFormProps) {
  const { regions } = useDeploymentRegions(team.id);
  const formState = useFormik({
    initialValues: {
      name: team.name,
      slug: team.slug,
      defaultRegion: team.defaultRegion ?? null,
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
        <div className="mb-6 flex flex-col gap-4">
          <Tooltip
            className="block max-w-[20rem]"
            tip={
              !canUpdate
                ? permissionDeniedTip(
                    "You do not have permission to update the team name.",
                    "team:update",
                  )
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
              disabled={!canUpdate || team.managedBy === "vercel"}
            />
          </Tooltip>
          <Tooltip
            className="block max-w-[20rem]"
            tip={
              !canUpdate
                ? permissionDeniedTip(
                    "You do not have permission to update the team slug.",
                    "team:update",
                  )
                : undefined
            }
          >
            <TextInput
              label="Team Slug"
              outerClassname="max-w-[20rem]"
              placeholder="Enter a slug for your team"
              onChange={formState.handleChange}
              value={formState.values.slug}
              // We hide the button when the tooltip is visible to avoid nesting buttons
              Icon={canUpdate ? CopyIcon : undefined}
              action={() => copyToClipboard(formState.values.slug)}
              id="slug"
              error={formState.errors.slug}
              disabled={!canUpdate}
            />
          </Tooltip>

          <DefaultRegionSelector
            value={formState.values.defaultRegion}
            onChange={(region) =>
              formState.setFieldValue("defaultRegion", region)
            }
            regions={regions}
            teamSlug={team.slug}
            disabledDueToPermissions={!canUpdate}
          />
        </div>

        <Button
          className="float-right"
          disabled={
            !formState.dirty ||
            formState.isSubmitting ||
            !formState.isValid ||
            !canUpdate
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
