import { Modal } from "@ui/Modal";
import { TextInput } from "@ui/TextInput";
import { Button } from "@ui/Button";
import { Loading } from "@ui/Loading";
import { ReactElement, useState } from "react";
import { useFormik } from "formik";
import * as Yup from "yup";
import { TeamResponse, CreateProjectResponse } from "generatedApi";
import { useCurrentTeam } from "api/teams";
import { useCreateProject } from "api/projects";
import { cn } from "@ui/cn";

export function useCreateProjectModal(): [
  ReactElement | null,
  (team?: TeamResponse) => void,
] {
  const [modalOpen, setModalOpen] = useState(false);
  const [team, setTeam] = useState<TeamResponse | undefined>();
  const currentTeam = useCurrentTeam();

  const selectedTeam = team || currentTeam;

  const modal = modalOpen ? (
    <Modal title="Create Project" onClose={() => setModalOpen(false)}>
      <>
        {selectedTeam && (
          <p className="mb-5">
            Create a project in{" "}
            <span className="font-semibold">{selectedTeam?.name}</span>.
          </p>
        )}
        {selectedTeam ? (
          <CreateProjectForm
            onClose={() => setModalOpen(false)}
            team={selectedTeam}
            onSuccess={(project) => {
              const projectUrl = `/t/${selectedTeam.slug}/${project.projectSlug}/${project.deploymentName}/data`;
              window.location.href = projectUrl;
            }}
          />
        ) : (
          <Loading />
        )}
      </>
    </Modal>
  ) : null;

  return [
    modal,
    (t?: TeamResponse) => {
      setModalOpen(true);
      setTeam(t);
    },
  ];
}

export const ProjectNameSchema = Yup.string()
  .min(3, "Project name must be at least 3 characters long.")
  .max(128, "Project name must be at most 128 characters long.")
  .required("Project name is required.");
const CreateProjectSchema = Yup.object().shape({
  projectName: ProjectNameSchema,
});

export function CreateProjectForm({
  onClose,
  team,
  showLabel = false,
  onSuccess,
}: {
  onClose(): void;
  team: TeamResponse;
  showLabel?: boolean;
  onSuccess: (project: CreateProjectResponse) => void;
}) {
  const createProject = useCreateProject(team.id);
  const formState = useFormik({
    initialValues: {
      projectName: "",
    },
    validateOnChange: false,
    validateOnBlur: true,
    validationSchema: CreateProjectSchema,
    onSubmit: async (values: { projectName: string }) => {
      const project = await createProject({
        ...values,
        team: team.slug,
        deploymentType: "dev",
      });
      onSuccess(project);
      onClose();
    },
  });
  return (
    <form
      onSubmit={formState.handleSubmit}
      aria-label="Create project"
      className="flex gap-2"
    >
      <TextInput
        labelHidden={!showLabel}
        label="Project Name"
        outerClassname="w-full"
        placeholder="Project name"
        onChange={(e) => {
          // Reset the errors so the user can blur the form
          formState.setErrors({});
          formState.handleChange(e);
        }}
        onBlur={formState.handleBlur}
        value={formState.values.projectName}
        autoFocus
        id="projectName"
        error={formState.touched ? formState.errors.projectName : undefined}
      />

      <Button
        disabled={!formState.dirty || !formState.isValid}
        className={cn("h-fit", showLabel ? "mt-6" : "")}
        size="sm"
        type="submit"
        aria-label="submit"
        loading={formState.isSubmitting}
      >
        {formState.isSubmitting ? "Creating" : "Create"}
      </Button>
    </form>
  );
}
