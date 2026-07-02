import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Sheet } from "@ui/Sheet";
import { TextInput } from "@ui/TextInput";
import { useFormik } from "formik";
import { TeamResponse } from "generatedApi";
import * as Yup from "yup";
import { permissionDeniedTip } from "elements/permissionDeniedTip";

export type TeamNameFormProps = {
  team: TeamResponse;
  onUpdateTeam: (body: { name: string }) => Promise<void>;
  canUpdate: boolean;
};

const TeamNameSchema = Yup.object().shape({
  name: Yup.string()
    .min(3, "Team name must be at least 3 characters long.")
    .max(128, "Team name must be at most 128 characters long.")
    .required("Team name is required."),
});

export function TeamNameForm({
  team,
  onUpdateTeam,
  canUpdate,
}: TeamNameFormProps) {
  const formState = useFormik({
    initialValues: {
      name: team.name,
    },
    enableReinitialize: true,
    validationSchema: TeamNameSchema,
    onSubmit: async (values) => {
      await onUpdateTeam({ name: values.name });
    },
  });

  const disabled = !canUpdate || team.managedBy === "vercel";

  return (
    <Sheet className="text-sm">
      <h3 className="mb-4">Team Name</h3>
      <form onSubmit={formState.handleSubmit} aria-label="Edit team name">
        <div className="mb-6">
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
              labelHidden
              outerClassname="max-w-[20rem]"
              placeholder="Enter a name for your team"
              onChange={formState.handleChange}
              value={formState.values.name}
              id="name"
              error={formState.errors.name}
              disabled={disabled}
            />
          </Tooltip>
        </div>

        <Button
          className="float-right"
          disabled={
            !formState.dirty ||
            formState.isSubmitting ||
            !formState.isValid ||
            disabled
          }
          type="submit"
          aria-label="Save team name"
        >
          Save
        </Button>
      </form>
    </Sheet>
  );
}
