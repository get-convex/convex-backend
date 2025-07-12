import { Button } from "@ui/Button";
import { useVerifyProfileEmail } from "api/profile";
import { LoginLayout } from "layouts/LoginLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";
import { useState } from "react";

export { getServerSideProps } from "lib/ssr";

function Verify() {
  const router = useRouter();
  const { code } = router.query;
  const [error, setError] = useState<string | undefined>();
  const verify = useVerifyProfileEmail(code as string);

  return (
    <LoginLayout>
      {error ? (
        <span
          role="alert"
          className="max-w-prose text-center text-balance text-content-primary"
        >
          {error}
        </span>
      ) : (
        <div className="flex flex-col items-center gap-6">
          To complete the email verification process, click the button below.
          <Button
            onClick={async () => {
              try {
                await verify();
              } catch (e: any) {
                if (
                  e.code === "InvalidVerificationCode" ||
                  e.code === "EmailAlreadyExists"
                ) {
                  setError(
                    "The provided email verification code is invalid, or the email has already been verified. Follow the link sent to your email to verify an email address.",
                  );
                } else {
                  setError("Failed to verify email.");
                }
              }
            }}
          >
            Verify Email
          </Button>
        </div>
      )}
    </LoginLayout>
  );
}

export default withAuthenticatedPage(Verify);
