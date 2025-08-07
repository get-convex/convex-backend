import { useAuth0 } from "hooks/useAuth0";
import {
  useListIdentities,
  useSetLinkIdentityCookie,
  useUnlinkIdentity,
  useChangePrimaryIdentity,
} from "api/profile";
import { useEffect, useState } from "react";
import { Sheet } from "@ui/Sheet";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { UserProfile } from "@auth0/nextjs-auth0/client";
import { AuthIdentityResponse } from "generatedApi";
import { LoadingTransition } from "@ui/Loading";
import GoogleLogo from "logos/google.svg";
import GithubLogo from "logos/github-logo.svg";
import VercelLogo from "logos/vercel.svg";
import { Tooltip } from "@ui/Tooltip";
import { useSessionStorage } from "react-use";
import { Button } from "@ui/Button";
import { Menu, MenuItem } from "@ui/Menu";
import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

export const linkIdentityStateKey = "linkIdentityState";
export type LinkIdentityState = {
  returnTo?: string;
};

export function ConnectedIdentities() {
  const { user } = useAuth0();
  const identities = useListIdentities();
  const unlinkIdentity = useUnlinkIdentity();
  const changePrimaryIdentity = useChangePrimaryIdentity();
  const [unlinkingId, setUnlinkingId] = useState<string | null>(null);
  const [changingPrimaryId, setChangingPrimaryId] = useState<string | null>(
    null,
  );
  const [error, setError] = useState<string | undefined>();
  const [, setLinkIdentityState] = useSessionStorage<LinkIdentityState>(
    linkIdentityStateKey,
    {},
  );
  const [providerToLink, setProviderToLink] = useState<string | null>(null);

  // Find which providers are already connected
  const connectedProviders = new Set(
    identities?.map((identity) => {
      let { provider } = identity;
      const { userId } = identity;
      if (provider === "oidc" && typeof userId === "string") {
        const [oidcProvider] = userId.split("|");
        provider = oidcProvider;
      }
      return provider;
    }) ?? [],
  );

  const setLinkIdentityCookie = useSetLinkIdentityCookie();

  const handleLinkClick = async (provider: string) => {
    await setLinkIdentityCookie();
    setLinkIdentityState({
      returnTo: "/profile",
    });
    setProviderToLink(provider);
  };

  // Handle this in a useEffect to make sure that the identity state had time
  // to propogate to session storage before redirecting.
  useEffect(() => {
    if (providerToLink) {
      let connection = providerToLink;
      if (providerToLink === "google") connection = "google-oauth2";
      window.location.href = `/api/auth/login?connection=${connection}&returnTo=/link_identity?resume=fromProfile&returnTo=/profile`;
    }
  }, [providerToLink]);

  // Find the identity being changed to primary
  const candidatePrimaryIdentity = identities?.find(
    (i) => i.userId === changingPrimaryId,
  );

  const { changePrimaryIdentity: changePrimaryIdentityFlag } =
    useLaunchDarkly();

  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Identities</h3>
      <p className="max-w-prose text-sm">
        These are the identities associated with your account. You can change
        your primary identity and unlink secondary identities here.
      </p>
      <LoadingTransition loadingProps={{ className: "h-[13rem]" }}>
        {user && identities && (
          <div className="flex w-full flex-col gap-4">
            <div className="flex flex-col">
              {identities?.map((identity) => {
                // user.sub is like "provider|id"; we want everything after provider|
                const primaryId = user?.sub?.substring(
                  user.sub.indexOf("|") + 1,
                );
                const isPrimary = identity.userId === primaryId;
                return (
                  <div
                    key={identity.userId}
                    className="flex flex-wrap items-center justify-between gap-4 border-b py-2 last:border-b-0"
                  >
                    <div className="flex min-w-0 flex-1 items-center gap-2">
                      <ProviderLogo
                        provider={identity.provider}
                        userId={identity.userId}
                      />
                      <span className="min-w-0 flex-1 text-sm">
                        <IdentityDisplayName
                          user={user}
                          isPrimary={isPrimary}
                          identity={identity}
                        />
                      </span>
                      {isPrimary && (
                        <div className="rounded-sm border p-1 text-xs">
                          Primary
                        </div>
                      )}
                    </div>
                    <Menu
                      placement="bottom-end"
                      buttonProps={{
                        variant: "neutral",
                        icon: <DotsVerticalIcon />,
                        "aria-label": "Identity options",
                        size: "xs",
                      }}
                    >
                      {changePrimaryIdentityFlag ? (
                        <MenuItem
                          action={() => setChangingPrimaryId(identity.userId)}
                          disabled={
                            isPrimary || identity.connection === "vercel"
                          }
                          tip={
                            isPrimary
                              ? "This is already your primary identity."
                              : identity.connection === "vercel"
                                ? "You cannot set a Vercel identity as your primary identity."
                                : undefined
                          }
                          tipSide="right"
                        >
                          Set as primary
                        </MenuItem>
                      ) : null}
                      <MenuItem
                        action={() => setUnlinkingId(identity.userId)}
                        disabled={isPrimary}
                        variant="danger"
                        tip={
                          isPrimary
                            ? changePrimaryIdentityFlag
                              ? "You cannot unlink your primary identity. To unlink this identity, you must first set a new primary identity."
                              : "You cannot unlink your primary identity."
                            : undefined
                        }
                        tipSide="right"
                      >
                        Unlink
                      </MenuItem>
                    </Menu>
                    {unlinkingId === identity.userId && (
                      <ConfirmationDialog
                        onClose={() => {
                          setUnlinkingId(null);
                          setError(undefined);
                        }}
                        onConfirm={async () => {
                          try {
                            await unlinkIdentity({
                              userId: identity.userId,
                              provider: identity.provider,
                            });
                            setUnlinkingId(null);
                            window.location.href = "/api/auth/logout";
                          } catch (e: any) {
                            setError(e.message);
                            throw e;
                          }
                        }}
                        confirmText="Unlink"
                        variant="danger"
                        dialogTitle="Unlink Identity"
                        dialogBody="Unlinking this identity will remove it from your account. You will not be able to use it to log in unless you link it again. You will be logged out of the dashboard after unlinking your identity. However, your other sessions will remain logged in. Be sure to log out of the CLI or Chef if necessary."
                        error={error}
                      />
                    )}
                  </div>
                );
              })}
            </div>
            <h4 className="mt-4">Link an additional account</h4>
            <p className="max-w-prose text-sm">
              You can add additional log in methods to your Convex account.
            </p>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                icon={<GithubLogo className="dark:fill-white" />}
                variant="neutral"
                className="w-fit"
                disabled={connectedProviders.has("github")}
                tip={
                  connectedProviders.has("github")
                    ? "You cannot link multiple GitHub accounts to Convex. Please contact support to merge your accounts."
                    : undefined
                }
                onClick={() => handleLinkClick("github")}
              >
                Link GitHub account
              </Button>
              <Button
                size="sm"
                icon={<GoogleLogo className="dark:fill-white" />}
                variant="neutral"
                className="w-fit"
                onClick={() => handleLinkClick("google-oauth2")}
              >
                Link Google account
              </Button>
            </div>
            {/* Render the confirmation dialog for changing primary identity at the root */}
            {changingPrimaryId && candidatePrimaryIdentity && (
              <ConfirmationDialog
                onClose={() => {
                  setChangingPrimaryId(null);
                  setError(undefined);
                }}
                onConfirm={async () => {
                  try {
                    await changePrimaryIdentity({
                      newPrimaryProvider: candidatePrimaryIdentity.provider,
                      newPrimaryUserId: candidatePrimaryIdentity.userId,
                    });
                    setChangingPrimaryId(null);
                    window.location.href = "/api/auth/logout";
                  } catch (e: any) {
                    setError(e.message);
                    throw e;
                  }
                }}
                confirmText="Confirm"
                variant="primary"
                dialogTitle="Change Primary Identity"
                dialogBody="Changing your primary identity will log you out of the dashboard. You will be logged out of the dashboard and need to log in again after changing your primary identity."
                error={error}
              />
            )}
          </div>
        )}
      </LoadingTransition>
    </Sheet>
  );
}

export function IdentityDisplayName({
  identity,
  user,
  isPrimary,
}: {
  user: UserProfile;
  identity: AuthIdentityResponse;
  isPrimary: boolean;
}) {
  let main: string | undefined;
  let { provider } = identity;
  let { userId } = identity;

  // Special handling for OIDC
  if (provider === "oidc" && typeof userId === "string") {
    const [oidcProvider, ...rest] = userId.split("|");
    if (rest.length > 0) {
      provider = oidcProvider;
      userId = rest[rest.length - 1];
    }
  }

  const profileData = isPrimary
    ? {
        email: user?.email,
        username: user?.nickname,
      }
    : identity.profileData;

  if (provider === "google-oauth2") {
    main = profileData.email ?? undefined;
  } else if (provider === "github") {
    main = profileData.username ?? profileData.nickname ?? undefined;
  } else if (provider === "vercel") {
    const [account, u] = userId.split(":user:");
    const accountId = account.split(":")[1];
    main = `account:${accountId.slice(0, 8)} user:${u.slice(0, 8)}`;
  } else {
    main = userId;
  }

  return <p className="max-w-full truncate">{main}</p>;
}

function ProviderLogo({
  provider,
  userId,
}: {
  provider: string;
  userId: string;
}) {
  // Special handling for OIDC
  let resolvedProvider = provider;
  if (provider === "oidc" && typeof userId === "string") {
    const [oidcProvider] = userId.split("|");
    resolvedProvider = oidcProvider;
  }
  const logo = (() => {
    switch (resolvedProvider) {
      case "google-oauth2":
        return (
          <div className="flex size-[1.75rem] min-w-[1.75rem] items-center justify-center">
            <GoogleLogo className="size-6" />
          </div>
        );
      case "github":
        return (
          <div className="flex size-[1.75rem] min-w-[1.75rem] items-center justify-center">
            <GithubLogo className="size-6 dark:fill-white" />
          </div>
        );
      case "vercel":
        return (
          <div className="flex size-[1.75rem] min-w-[1.75rem] items-center justify-center">
            <VercelLogo className="size-6 dark:fill-white" />
          </div>
        );
      default:
        return (
          <div className="rounded-sm border p-1 text-xs opacity-60">
            {providerToDisplayName[resolvedProvider] || resolvedProvider}
          </div>
        );
    }
  })();

  return (
    <Tooltip tip={providerToDisplayName[resolvedProvider] || resolvedProvider}>
      {logo}
    </Tooltip>
  );
}

export const providerToDisplayName: Record<string, string> = {
  "google-oauth2": "Google",
  github: "GitHub",
  vercel: "Vercel",
};
