import { useRouter } from "next/router";
import { useAuthHeader } from "hooks/fetching";
import { useEffect, useState } from "react";
import { LoginLayout } from "layouts/LoginLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { Sheet } from "dashboard-common/elements/Sheet";
import { useProfileEmails } from "api/profile";
import { EmailList } from "components/profile/EmailList";
import { useAcceptInvite } from "api/invitations";
import {
  LoadingTransition,
  LoadingLogo,
} from "dashboard-common/elements/Loading";

export { getServerSideProps } from "lib/ssr";

const messagesForError: Record<
  "EmailNotVerified" | "Internal" | "Generic",
  string
> = {
  EmailNotVerified:
    "The email address associated with this team invitation has not been added to your account.",
  Internal: "Failed to accept invitation.",
  Generic:
    "This Convex team invitation has expired or does not exist. If you're attempting to join a team, ask a team admin to resend the invite.",
};

function AcceptInvite() {
  const authHeader = useAuthHeader();
  const router = useRouter();
  const [didRequest, setDidRequest] = useState(false);
  const [error, setError] = useState<
    "EmailNotVerified" | "Internal" | "Generic" | undefined
  >();

  const emails = useProfileEmails();
  const acceptInvititation = useAcceptInvite(router.query.inviteId as string);

  useEffect(() => {
    if (
      didRequest ||
      typeof router.query.inviteId !== "string" ||
      !authHeader
    ) {
      return;
    }

    const acceptInvite = async () => {
      try {
        const result = await acceptInvititation();
        window.location.href = `/t/${result.slug}`;
      } catch (e: any) {
        const code = e?.code || undefined;
        if (code === "MemberAlreadyOnTeam") {
          // You're already on the team, redirect to the teams page.
          window.location.href = "/";
          return;
        }
        if (code === "EmailNotVerified") {
          setError("EmailNotVerified");
          return;
        }
        if (code === "InvitationNotFound") {
          setError("Generic");
          return;
        }

        setError("Internal");
      }
    };

    setDidRequest(true);
    void acceptInvite();
  }, [
    router.query.inviteId,
    authHeader,
    router,
    didRequest,
    acceptInvititation,
  ]);

  return (
    <LoginLayout>
      {error ? (
        <Sheet className="flex flex-col gap-6">
          <span role="alert" className="max-w-prose text-content-primary">
            {messagesForError[error]}
          </span>
          {error === "EmailNotVerified" && (
            <>
              <span className="flex gap-1">
                Add your email address, verify it, and try again.
              </span>
              The following email addresses are associated with your account:
              <LoadingTransition>
                {emails && (
                  <div className="w-full">
                    <EmailList emails={emails} />
                  </div>
                )}
              </LoadingTransition>
            </>
          )}
        </Sheet>
      ) : (
        <LoadingLogo />
      )}
    </LoginLayout>
  );
}

function Main() {
  return <AcceptInvite />;
}

export default withAuthenticatedPage(Main);
