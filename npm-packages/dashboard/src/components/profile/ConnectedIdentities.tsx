import { useState } from "react";
import { useIdentities, useUnlinkIdentity } from "api/profile";
import { Sheet } from "@ui/Sheet";
import { IdentityResponse } from "generatedApi";
import { LoadingTransition } from "@ui/Loading";
import GoogleLogo from "logos/google.svg";
import GithubLogo from "logos/github-logo.svg";
import VercelLogo from "logos/vercel.svg";
import { Tooltip } from "@ui/Tooltip";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";

export function ConnectedIdentities() {
  const identities = useIdentities();
  const unlinkIdentity = useUnlinkIdentity();
  const [unlinkingId, setUnlinkingId] = useState<string | null>(null);
  const [confirmUnlinkId, setConfirmUnlinkId] = useState<string | null>(null);

  const handleUnlinkClick = (identityId: string) => {
    setConfirmUnlinkId(identityId);
  };

  const handleConfirmUnlink = async () => {
    if (!confirmUnlinkId) return;

    setUnlinkingId(confirmUnlinkId);
    setConfirmUnlinkId(null);
    try {
      await unlinkIdentity({ userId: confirmUnlinkId });
      window.location.href = "/api/auth/logout";
    } finally {
      setUnlinkingId(null);
    }
  };

  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Identities</h3>
      <p className="max-w-prose text-sm">
        These are the identities associated with your account.
      </p>
      <p>
        Identities are used to login to Convex, and are distinct from the emails
        connected to your account for communication purposes. However, you can
        only login with identities that are verified emails on your Convex
        account.
      </p>
      <LoadingTransition loadingProps={{ className: "h-[13rem]" }}>
        {identities && (
          <div className="flex w-full flex-col gap-4">
            {identities.map((identity) => (
              <IdentityCard
                key={identity.id}
                identity={identity}
                canUnlink={identities.length > 1}
                onUnlink={() => handleUnlinkClick(identity.id)}
                isUnlinking={unlinkingId === identity.id}
              />
            ))}
          </div>
        )}
      </LoadingTransition>

      {confirmUnlinkId !== null && (
        <ConfirmationDialog
          dialogTitle="Unlink Identity"
          dialogBody={
            <div className="flex flex-col gap-1">
              <p>Are you sure you want to unlink this identity? </p>
              <p>
                After you unlink this identity, you must also delete the
                associated email to restrict this email from logging in to
                Convex.
              </p>
              <p>
                Once you unlink this identity, you will be logged out of the
                dashboard.
              </p>
            </div>
          }
          confirmText="Unlink"
          onConfirm={handleConfirmUnlink}
          onClose={() => setConfirmUnlinkId(null)}
        />
      )}
    </Sheet>
  );
}

function IdentityCard({
  identity,
  canUnlink,
  onUnlink,
  isUnlinking,
}: {
  identity: IdentityResponse;
  canUnlink: boolean;
  onUnlink: () => void;
  isUnlinking: boolean;
}) {
  return (
    <div className="flex items-start justify-between gap-4 rounded-lg border p-4">
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        {/* Email or User ID */}
        <div className="min-w-0">
          {identity.email ? (
            <span className="text-sm font-medium">{identity.email}</span>
          ) : (
            <div className="flex items-center gap-2">
              <span className="font-mono text-sm text-content-secondary">
                {identity.id}
              </span>
              <Tooltip tip="Email could not be retrieved from identity provider">
                <InfoCircledIcon className="h-4 w-4 text-content-tertiary" />
              </Tooltip>
            </div>
          )}
        </div>

        {/* Provider icons */}
        <div className="flex space-x-1">
          {identity.providers.map((provider) => (
            <div key={provider} className="relative">
              <ProviderLogo provider={provider} />
            </div>
          ))}
        </div>
      </div>

      {/* Unlink button */}
      <div className="flex-shrink-0">
        <Button
          variant="danger"
          size="xs"
          onClick={onUnlink}
          loading={isUnlinking}
          disabled={!canUnlink}
          tip={canUnlink ? undefined : "You cannot unlink your only identity"}
        >
          Unlink
        </Button>
      </div>
    </div>
  );
}

function ProviderLogo({ provider }: { provider: string }) {
  const logo = (() => {
    switch (provider) {
      case "google":
        return (
          <div className="flex size-10 min-w-10 items-center justify-center rounded-full border bg-white">
            <GoogleLogo className="size-6" />
          </div>
        );
      case "github":
        return (
          <div className="flex size-10 min-w-10 items-center justify-center rounded-full border bg-white">
            <GithubLogo className="size-6 dark:fill-black" />
          </div>
        );
      case "vercel":
        return (
          <div className="flex size-10 min-w-10 items-center justify-center rounded-full border bg-white">
            <VercelLogo className="size-6 dark:fill-black" />
          </div>
        );
      default:
        return (
          <div className="flex size-10 min-w-10 items-center justify-center rounded-full border bg-gray-100">
            <span className="text-sm font-medium uppercase">
              {provider.slice(0, 2)}
            </span>
          </div>
        );
    }
  })();

  return (
    <Tooltip tip={providerToDisplayName[provider] || provider}>{logo}</Tooltip>
  );
}

const providerToDisplayName: Record<string, string> = {
  google: "Google",
  github: "GitHub",
  vercel: "Vercel",
};
