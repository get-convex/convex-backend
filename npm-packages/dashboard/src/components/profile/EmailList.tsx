import { PlusIcon } from "@radix-ui/react-icons";
import { Button } from "dashboard-common/elements/Button";
import { Modal } from "dashboard-common/elements/Modal";
import { useState } from "react";
import { MemberEmailResponse } from "generatedApi";
import { EmailCreateForm } from "./EmailCreateForm";
import { EmailListItem } from "./EmailListItem";

export function EmailList({ emails }: { emails: MemberEmailResponse[] }) {
  const [showAddModal, setShowAddModal] = useState(false);

  return (
    <>
      <div className="flex flex-col">
        {emails
          .sort((a, b) => {
            if (a.isPrimary) return -1;
            if (b.isPrimary) return 1;
            if (a.isVerified && !b.isVerified) return -1;
            if (!a.isVerified && b.isVerified) return 1;
            return 0;
          })
          .map((email) => (
            <EmailListItem email={email} key={email.id} />
          ))}
      </div>
      <Button
        icon={<PlusIcon />}
        variant="neutral"
        className="mt-2 w-fit"
        onClick={() => setShowAddModal(true)}
      >
        Add email
      </Button>
      {showAddModal && (
        <Modal onClose={() => setShowAddModal(false)} title="Add email">
          <p className="mb-5">
            Add an email to your Convex account. Once verified, this email may
            be used to accept team invitations.
          </p>
          <EmailCreateForm
            emails={emails}
            onCreate={() => {
              setShowAddModal(false);
            }}
          />
        </Modal>
      )}
    </>
  );
}
