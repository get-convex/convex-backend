import { LoginLayout } from "layouts/LoginLayout";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import React from "react";
import { UserProfile } from "@auth0/nextjs-auth0/client";
import { UIProvider } from "@ui/UIContext";

export function LinkIdentityNoMultipleIdentities({
  user,
}: {
  user: UserProfile | undefined;
}) {
  return (
    <div className="h-screen">
      <LoginLayout>
        <Sheet>
          <h2 className="mb-4">Link Existing Account</h2>
          <p className="mb-2 max-w-prose">
            The account associated with{" "}
            <span className="font-semibold">{user?.email}</span> is already
            linked to a Convex account.
          </p>
          <p className="max-w-prose">
            If you need to merge your accounts, please contact us at{" "}
            <a
              className="text-content-link hover:underline"
              href="mailto:support@convex.dev"
            >
              support@convex.dev
            </a>{" "}
            to get help accessing your account.
          </p>
          <UIProvider>
            <Button
              href="/api/auth/logout"
              className="mt-4 w-fit"
              variant="neutral"
            >
              Log Out
            </Button>
          </UIProvider>
        </Sheet>
      </LoginLayout>
    </div>
  );
}
