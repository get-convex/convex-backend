import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { useFormik } from "formik";
import { RegionName, TeamResponse } from "generatedApi";
import { useDeploymentRegions } from "api/deployments";
import { DefaultRegionSelector } from "./DefaultRegionSelector";

export type DefaultRegionFormProps = {
  team: TeamResponse;
  onUpdateTeam: (body: { defaultRegion: RegionName | null }) => Promise<void>;
  canUpdate: boolean;
};

export function DefaultRegionForm({
  team,
  onUpdateTeam,
  canUpdate,
}: DefaultRegionFormProps) {
  const { regions } = useDeploymentRegions(team.id);
  const formState = useFormik<{ defaultRegion: RegionName | null }>({
    initialValues: {
      defaultRegion: team.defaultRegion ?? null,
    },
    enableReinitialize: true,
    onSubmit: async (values) => {
      await onUpdateTeam({ defaultRegion: values.defaultRegion });
    },
  });

  return (
    <Sheet className="text-sm">
      <h3 className="mb-1">Default Region</h3>
      <p className="mb-4 max-w-prose text-content-secondary">
        The region where new deployments in this team are created.
      </p>
      <form onSubmit={formState.handleSubmit} aria-label="Edit default region">
        <div className="mb-6">
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
          aria-label="Save default region"
        >
          Save
        </Button>
      </form>
    </Sheet>
  );
}
