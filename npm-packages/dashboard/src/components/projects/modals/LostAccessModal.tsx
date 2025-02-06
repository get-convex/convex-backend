import { CodeLine } from "elements/CodeLine";
import { Modal } from "dashboard-common/elements/Modal";

export function LostAccessModal({
  onClose,
  teamSlug,
  projectSlug,
}: {
  onClose: () => void;
  teamSlug: string;
  projectSlug: string;
}) {
  return (
    <Modal title="Lost Access" onClose={onClose}>
      <>
        <LostAccessDescription />
        <LostAccessCommand teamSlug={teamSlug} projectSlug={projectSlug} />
      </>
    </Modal>
  );
}

export function LostAccessDescription() {
  return (
    <p className="mb-5 text-sm leading-6 text-content-primary">
      Reinitialize a Convex app in your local directory if you've lost your{" "}
      <code className="rounded bg-background-tertiary p-1 text-content-primary">
        .env.local
      </code>{" "}
      file.
    </p>
  );
}

export function LostAccessCommand({
  teamSlug,
  projectSlug,
}: {
  teamSlug: string;
  projectSlug: string;
}) {
  const cliCommand = `npx convex dev --configure=existing --team ${teamSlug} --project ${projectSlug}`;
  return <CodeLine className="text-xs" code={cliCommand} />;
}
