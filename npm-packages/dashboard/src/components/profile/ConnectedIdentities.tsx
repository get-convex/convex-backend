import { useListIdentities } from "api/profile";
import Link from "next/link";
import { Sheet } from "@ui/Sheet";
import { AuthIdentityResponse } from "generatedApi";
import { LoadingTransition } from "@ui/Loading";
import GoogleLogo from "logos/google.svg";
import GithubLogo from "logos/github-logo.svg";
import VercelLogo from "logos/vercel.svg";
import { Tooltip } from "@ui/Tooltip";
import { InfoCircledIcon } from "@radix-ui/react-icons";

type IdentityGroup = {
  parentUserId: string | null;
  identities: AuthIdentityResponse[];
  isPrimary: boolean;
};

export function ConnectedIdentities() {
  const identities = useListIdentities();
  // Group identities by parent user ID
  const identityGroups = identities ? groupIdentities(identities) : [];

  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Identities</h3>
      <p className="max-w-prose text-sm">
        These are the identities associated with your account. Please{" "}
        <Link
          className="text-content-link hover:underline"
          href="mailto:support@convex.dev"
        >
          contact support
        </Link>{" "}
        if you would like to remove an identity from your account.
      </p>
      <LoadingTransition loadingProps={{ className: "h-[13rem]" }}>
        {identities && (
          <div className="flex w-full flex-col gap-4">
            <div className="flex flex-col gap-4">
              {identityGroups.map((group, groupIndex) => (
                <IdentityGroupRenderer
                  key={group.parentUserId || `group-${groupIndex}`}
                  group={group}
                />
              ))}
            </div>
          </div>
        )}
      </LoadingTransition>
    </Sheet>
  );
}

function IdentityGroupRenderer({ group }: { group: IdentityGroup }) {
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
            <Tooltip tip="These identities are grouped because they shared the same email address at the time they were linked.">
              <InfoCircledIcon className="h-4 w-4 text-gray-500" />
            </Tooltip>
          )}
          {group.isPrimary && (
            <div className="rounded-sm border p-1 text-xs">Primary</div>
          )}
        </div>
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
    </div>
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
