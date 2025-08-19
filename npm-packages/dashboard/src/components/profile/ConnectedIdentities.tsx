import {
  useListIdentities,
  useUnlinkIdentity,
  useChangePrimaryIdentity,
} from "api/profile";
import { useState } from "react";
import { Sheet } from "@ui/Sheet";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { AuthIdentityResponse } from "generatedApi";
import { LoadingTransition } from "@ui/Loading";
import GoogleLogo from "logos/google.svg";
import GithubLogo from "logos/github-logo.svg";
import VercelLogo from "logos/vercel.svg";
import { Tooltip } from "@ui/Tooltip";
import { Menu, MenuItem } from "@ui/Menu";
import { DotsVerticalIcon, InfoCircledIcon } from "@radix-ui/react-icons";

type IdentityGroup = {
  parentUserId: string | null;
  identities: AuthIdentityResponse[];
  isPrimary: boolean;
};

export function ConnectedIdentities() {
  const identities = useListIdentities();
  const unlinkIdentity = useUnlinkIdentity();
  const changePrimaryIdentity = useChangePrimaryIdentity();
  const [unlinkingId, setUnlinkingId] = useState<string | null>(null);
  const [changingPrimaryId, setChangingPrimaryId] = useState<string | null>(
    null,
  );
  const [error, setError] = useState<string | undefined>();
  // Group identities by parent user ID
  const identityGroups = identities ? groupIdentities(identities) : [];

  // Find the identity being changed to primary
  const candidatePrimaryIdentity = identities?.find(
    (i) => i.userId === changingPrimaryId,
  );

  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Identities</h3>
      <p className="max-w-prose text-sm">
        These are the identities associated with your account. You can change
        your primary identity and unlink secondary identities here.
      </p>
      <LoadingTransition loadingProps={{ className: "h-[13rem]" }}>
        {identities && (
          <div className="flex w-full flex-col gap-4">
            <div className="flex flex-col gap-4">
              {identityGroups.map((group, groupIndex) => (
                <IdentityGroupRenderer
                  key={group.parentUserId || `group-${groupIndex}`}
                  group={group}
                  unlinkingId={unlinkingId}
                  setUnlinkingId={setUnlinkingId}
                  setChangingPrimaryId={setChangingPrimaryId}
                  unlinkIdentity={unlinkIdentity}
                  error={error}
                  setError={setError}
                />
              ))}
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

function IdentityGroupRenderer({
  group,
  unlinkingId,
  setUnlinkingId,
  setChangingPrimaryId,
  unlinkIdentity,
  error,
  setError,
}: {
  group: IdentityGroup;
  unlinkingId: string | null;
  setUnlinkingId: (id: string | null) => void;
  setChangingPrimaryId: (id: string | null) => void;
  unlinkIdentity: any;
  error: string | undefined;
  setError: (error: string | undefined) => void;
}) {
  // If this is a single identity (no grouping), render normally
  if (group.identities.length === 1 && !group.parentUserId) {
    const identity = group.identities[0];
    const { isPrimary } = identity;

    return (
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-lg border p-3 px-2">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <ProviderLogo provider={identity.provider} userId={identity.userId} />
          <span className="min-w-0 flex-1 text-sm">
            <IdentityDisplayName identity={identity} />
          </span>
          {isPrimary && (
            <div className="rounded-sm border p-1 text-xs">Primary</div>
          )}
        </div>
        <IdentityMenu
          identity={identity}
          setChangingPrimaryId={setChangingPrimaryId}
          setUnlinkingId={setUnlinkingId}
        />
        <IdentityDialogs
          identity={identity}
          unlinkingId={unlinkingId}
          setUnlinkingId={setUnlinkingId}
          unlinkIdentity={unlinkIdentity}
          error={error}
          setError={setError}
        />
      </div>
    );
  }

  // Render grouped identities
  const secondaryIdentities = group.identities.filter((i) => !i.isPrimary);

  return (
    <div className="rounded-lg border p-3 px-2 py-2">
      {/* Primary/Main identity row */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <span className="font-semibold">{group.parentUserId}</span>
          {group.identities.length > 1 && (
            <Tooltip tip="These identities are grouped because they shared the same email address at the time they were linked. Unlinking one of these identities will unlink them all.">
              <InfoCircledIcon className="h-4 w-4 text-gray-500" />
            </Tooltip>
          )}
          {group.isPrimary && (
            <div className="rounded-sm border p-1 text-xs">Primary</div>
          )}
        </div>
        <GroupIdentityMenu
          group={group}
          setChangingPrimaryId={setChangingPrimaryId}
          setUnlinkingId={setUnlinkingId}
        />
      </div>

      {/* Secondary identities in group */}
      {secondaryIdentities.length > 0 && (
        <div className="mt-2 space-y-1">
          {secondaryIdentities.map((identity) => (
            <div
              key={identity.userId}
              className="flex items-center justify-between gap-4 text-sm"
            >
              <div className="flex min-w-0 flex-1 items-center gap-2">
                <ProviderLogo
                  provider={identity.provider}
                  userId={identity.userId}
                />
                <span className="min-w-0 flex-1">
                  <IdentityDisplayName identity={identity} />
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Dialogs for group or individual identities */}
      {group.parentUserId ? (
        // Group unlinking dialog
        <GroupUnlinkDialog
          group={group}
          unlinkingId={unlinkingId}
          setUnlinkingId={setUnlinkingId}
          unlinkIdentity={unlinkIdentity}
          error={error}
          setError={setError}
        />
      ) : (
        // Individual identity dialogs
        group.identities.map((identity) => (
          <IdentityDialogs
            key={identity.userId}
            identity={identity}
            unlinkingId={unlinkingId}
            setUnlinkingId={setUnlinkingId}
            unlinkIdentity={unlinkIdentity}
            error={error}
            setError={setError}
          />
        ))
      )}
    </div>
  );
}

function GroupUnlinkDialog({
  group,
  unlinkingId,
  setUnlinkingId,
  unlinkIdentity,
  error,
  setError,
}: {
  group: IdentityGroup;
  unlinkingId: string | null;
  setUnlinkingId: (id: string | null) => void;
  unlinkIdentity: any;
  error: string | undefined;
  setError: (error: string | undefined) => void;
}) {
  return (
    <>
      {unlinkingId === group.parentUserId && (
        <ConfirmationDialog
          onClose={() => {
            setUnlinkingId(null);
            setError(undefined);
          }}
          onConfirm={async () => {
            try {
              // Use the parentUserId as the userId, and pass empty provider since it will be ignored
              await unlinkIdentity({
                userId: group.parentUserId,
                provider: "",
              });
              setUnlinkingId(null);
              window.location.href = "/api/auth/logout";
            } catch (e: any) {
              setError(e.message);
              throw e;
            }
          }}
          confirmText="Unlink group"
          variant="danger"
          dialogTitle="Unlink Identity Group"
          dialogBody="Unlinking this group will remove all associated identities from your account. You will not be able to use any of these identities to log in unless you link them again. You will be logged out of the dashboard after unlinking. However, your other sessions will remain logged in. Be sure to log out of the CLI or Chef if necessary."
          error={error}
        />
      )}
    </>
  );
}

function GroupIdentityMenu({
  group,
  setChangingPrimaryId,
  setUnlinkingId,
}: {
  group: IdentityGroup;
  setChangingPrimaryId: (id: string | null) => void;
  setUnlinkingId: (id: string | null) => void;
}) {
  // For groups, we offer options for non-primary identities in the group
  const nonPrimaryIdentities = group.identities.filter((i) => !i.isPrimary);
  const primaryIdentity = group.identities.find((i) => i.isPrimary);

  return (
    <Menu
      placement="bottom-end"
      buttonProps={{
        variant: "neutral",
        icon: <DotsVerticalIcon />,
        "aria-label": "Identity group options",
        size: "xs",
      }}
    >
      {nonPrimaryIdentities.length > 0 ? (
        <>
          {(() => {
            const firstNonPrimary = nonPrimaryIdentities.find(
              (identity) => identity.connection !== "vercel",
            );
            return firstNonPrimary ? (
              <MenuItem
                key={`primary-${firstNonPrimary.userId}`}
                action={() => setChangingPrimaryId(firstNonPrimary.userId)}
                tip="Set first identity in group as primary"
                tipSide="right"
              >
                Set as primary
              </MenuItem>
            ) : null;
          })()}
        </>
      ) : null}

      {/* Unlink options for non-primary identities */}
      {nonPrimaryIdentities.length > 0 ? (
        <>
          {group.parentUserId ? (
            // If there's a parentUserId (WorkOS group), show single unlink option for the whole group
            <MenuItem
              key={`unlink-group-${group.parentUserId}`}
              action={() => setUnlinkingId(group.parentUserId)}
              variant="danger"
              tip="Unlink all identities in this group"
              tipSide="right"
            >
              Unlink group
            </MenuItem>
          ) : (
            // Individual unlink options for non-grouped identities
            nonPrimaryIdentities.map((identity) => (
              <MenuItem
                key={`unlink-${identity.userId}`}
                action={() => setUnlinkingId(identity.userId)}
                variant="danger"
                tip={`Unlink ${identity.provider} identity`}
                tipSide="right"
              >
                Unlink {identity.provider}
              </MenuItem>
            ))
          )}
        </>
      ) : null}

      {/* If only primary identity exists, show disabled unlink option */}
      {nonPrimaryIdentities.length === 0 && primaryIdentity ? (
        <MenuItem
          action={() => {}}
          disabled
          variant="danger"
          tip="You cannot unlink your primary identity. To unlink this identity, you must first set a new primary identity."
          tipSide="right"
        >
          Unlink
        </MenuItem>
      ) : null}
    </Menu>
  );
}

function IdentityMenu({
  identity,
  setChangingPrimaryId,
  setUnlinkingId,
}: {
  identity: AuthIdentityResponse;
  setChangingPrimaryId: (id: string | null) => void;
  setUnlinkingId: (id: string | null) => void;
}) {
  const { isPrimary } = identity;

  return (
    <Menu
      placement="bottom-end"
      buttonProps={{
        variant: "neutral",
        icon: <DotsVerticalIcon />,
        "aria-label": "Identity options",
        size: "xs",
      }}
    >
      <MenuItem
        action={() => setChangingPrimaryId(identity.userId)}
        disabled={isPrimary || identity.connection === "vercel"}
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
      <MenuItem
        action={() => setUnlinkingId(identity.userId)}
        disabled={isPrimary}
        variant="danger"
        tip={
          isPrimary
            ? "You cannot unlink your primary identity. To unlink this identity, you must first set a new primary identity."
            : undefined
        }
        tipSide="right"
      >
        Unlink
      </MenuItem>
    </Menu>
  );
}

function IdentityDialogs({
  identity,
  unlinkingId,
  setUnlinkingId,
  unlinkIdentity,
  error,
  setError,
}: {
  identity: AuthIdentityResponse;
  unlinkingId: string | null;
  setUnlinkingId: (id: string | null) => void;
  unlinkIdentity: any;
  error: string | undefined;
  setError: (error: string | undefined) => void;
}) {
  return (
    <>
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
    </>
  );
}

export function IdentityDisplayName({
  identity,
}: {
  identity: AuthIdentityResponse;
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

  const { profileData } = identity;

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

  return <p className="max-w-full truncate">{main || userId}</p>;
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

const providerToDisplayName: Record<string, string> = {
  "google-oauth2": "Google",
  github: "GitHub",
  vercel: "Vercel",
};

// Group identities by parent user ID
function groupIdentities(identities: AuthIdentityResponse[]): IdentityGroup[] {
  // First, deduplicate identities by userId to prevent duplicates from appearing
  const uniqueIdentities = identities.filter(
    (identity, index, arr) =>
      arr.findIndex((other) => other.userId === identity.userId) === index,
  );

  const groups = new Map<string, AuthIdentityResponse[]>();
  const individualGroups: IdentityGroup[] = [];

  // Group by parentUserId, but keep identities without parentUserId as individuals
  uniqueIdentities.forEach((identity) => {
    if (identity.parentUserId) {
      // Group identities with the same parentUserId
      if (!groups.has(identity.parentUserId)) {
        groups.set(identity.parentUserId, []);
      }
      groups.get(identity.parentUserId)!.push(identity);
    } else {
      // Keep identities without parentUserId as individual groups
      individualGroups.push({
        parentUserId: null,
        identities: [identity],
        isPrimary: identity.isPrimary,
      });
    }
  });

  // Convert grouped identities to IdentityGroup format
  const groupedResults = Array.from(groups.entries()).map(
    ([parentUserId, groupedIdentities]) => ({
      parentUserId,
      identities: groupedIdentities.sort((a, b) => {
        // Primary identity first, then by provider name
        if (a.isPrimary !== b.isPrimary) return a.isPrimary ? -1 : 1;
        return a.provider.localeCompare(b.provider);
      }),
      isPrimary: groupedIdentities.some((identity) => identity.isPrimary),
    }),
  );

  // Combine grouped and individual results
  return [...groupedResults, ...individualGroups];
}
