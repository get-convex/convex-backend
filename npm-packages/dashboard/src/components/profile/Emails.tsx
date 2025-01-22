import { Sheet } from "dashboard-common";
import { MemberEmailResponse } from "generatedApi";
import { EmailList } from "./EmailList";

export function Emails({ emails }: { emails: MemberEmailResponse[] }) {
  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Emails</h3>
      <p className="max-w-prose">
        The emails associated with your account are used to accept team
        invitations. Account-related communications will be sent to your primary
        email.
      </p>
      <EmailList emails={emails} />
    </Sheet>
  );
}
