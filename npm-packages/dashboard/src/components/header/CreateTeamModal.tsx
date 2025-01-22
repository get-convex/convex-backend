import { Modal } from "elements/Modal";
import { CreateTeamForm } from "./CreateTeamForm";

export function CreateTeamModal({ onClose }: { onClose(): void }) {
  return (
    <Modal title="Create Team" onClose={onClose}>
      <p className="mb-5">
        Collaborate with your team members by creating a Convex Team.
      </p>
      <CreateTeamForm onClose={onClose} />
    </Modal>
  );
}
