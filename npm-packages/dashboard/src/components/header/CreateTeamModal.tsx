import { Modal } from "@ui/Modal";
import { LocalDevCallout } from "@common/elements/LocalDevCallout";
import { useProfile } from "api/profile";
import { CreateTeamForm } from "./CreateTeamForm";

export function CreateTeamModal({ onClose }: { onClose(): void }) {
  const profile = useProfile();

  return (
    <Modal title="Create Team" onClose={onClose}>
      <p className="mb-5">
        Collaborate with your team members by creating a Convex Team.
      </p>
      <CreateTeamForm onClose={onClose} />
      <LocalDevCallout
        tipText="Tip: Run this to increase the number of teams you can create:"
        command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --member-entitlement max_teams 500 --member-id ${profile?.id ?? "{MEMBER_ID}"} --reason "local" --for-real`}
      />
    </Modal>
  );
}
