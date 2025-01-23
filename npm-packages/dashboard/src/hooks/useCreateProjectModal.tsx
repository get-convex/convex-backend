import { Modal, TextInput, Button, Loading, Spinner } from "dashboard-common";
import { ReactElement, useState } from "react";
import { useRouter } from "next/router";
import { useFormik } from "formik";
import * as Yup from "yup";
import { Team } from "generatedApi";
import { useCurrentTeam } from "api/teams";
import { useCreateProject } from "api/projects";

export function useCreateProjectModal(): [
  ReactElement | null,
  (team?: Team) => void,
] {
  const [modalOpen, setModalOpen] = useState(false);
  const [team, setTeam] = useState<Team | undefined>();
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
          />
        ) : (
          <Loading />
        )}
      </>
    </Modal>
  ) : null;

  return [
    modal,
    (t?: Team) => {
      setModalOpen(true);
      setTeam(t);
    },
  ];
}

const CreateProjectSchema = Yup.object().shape({
  projectName: Yup.string()
    .min(3, "Project name must be at least 3 characters long.")
    .max(128, "Project name must be at most 128 characters long.")
    .required("Project name is required."),
});

function CreateProjectForm({ onClose, team }: { onClose(): void; team: Team }) {
  const router = useRouter();
  const createProject = useCreateProject(team.id);
  const formState = useFormik({
    initialValues: {
      projectName: "",
    },
    validationSchema: CreateProjectSchema,
    onSubmit: async (values: { projectName: string }) => {
      const project = await createProject({
        ...values,
        team: team.slug,
        deploymentType: "dev",
      });
      const projectUrl = `/t/${team.slug}/${project.projectSlug}/${project.deploymentName}/data`;
      await router.push(
        {
          pathname: projectUrl,
        },
        {
          pathname: projectUrl,
        },
      );
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
        labelHidden
        outerClassname="w-full"
        placeholder="Project name"
        onChange={formState.handleChange}
        value={formState.values.projectName}
        autoFocus
        id="projectName"
        error={formState.errors.projectName}
      />

      <Button
        disabled={
          !formState.dirty || formState.isSubmitting || !formState.isValid
        }
        className="mt-[1px] h-fit"
        size="sm"
        type="submit"
        aria-label="submit"
        icon={formState.isSubmitting ? <Spinner /> : undefined}
      >
        {formState.isSubmitting ? "Creating" : "Create"}
      </Button>
    </form>
  );
}
