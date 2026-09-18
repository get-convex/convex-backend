import { useState } from "react";
import {
  DotsVerticalIcon,
  ExclamationTriangleIcon,
  ExternalLinkIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Link } from "@ui/Link";
import { LoadingTransition } from "@ui/Loading";
import { Menu, MenuItem } from "@ui/Menu";
import { Tooltip } from "@ui/Tooltip";
import { cn } from "@ui/cn";
import type { SsoOrganizationDomain, TeamResponse } from "generatedApi";
import {
  useDeleteTeamDomain,
  useDomainPortalLink,
  useTeamDomains,
} from "api/domains";
import { useProfileEmails } from "api/profile";
import {
  useHasCustomRolePermission,
  useIsCurrentMemberTeamAdmin,
} from "api/roles";
import { useTeamEntitlements } from "api/teams";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { NoPermissionMessage } from "elements/NoPermissionMessage";
import { permissionDeniedTip } from "elements/permissionDeniedTip";
import { TEAM_RESOURCE } from "lib/permissions";
import { EmptyStateRow, SettingsSheet } from "./SettingsSheet";
import { DomainStatusBadge } from "./StatusBadge";

// A domain serves both products, but directory sync is still behind a flag, so
// the sheet only names what the member can actually reach from here.
function domainCopy(directorySync: boolean) {
  return {
    description: "These domains are used to manage team members.",
    descriptionTip: directorySync
      ? "Team members can only be managed by SSO or Directory Sync if their account is associated with an email domain that has been verified by this team."
      : "Team members can only be managed by SSO if their account is associated with an email domain that has been verified by this team.",
    unentitled: directorySync
      ? "SSO and Directory Sync are not available on your plan."
      : "SSO is not available on your plan.",
    empty: directorySync
      ? "No domains have been configured. Start by verifying a domain to configure Single sign-on or Directory Sync."
      : "No domains have been configured. Start by verifying a domain to configure Single sign-on.",
  };
}

export function DomainsSheet({ team }: { team: TeamResponse }) {
  const { directorySync } = useLaunchDarkly();
  const copy = domainCopy(directorySync);
  const isTeamAdmin = useIsCurrentMemberTeamAdmin();
  const canView = useHasCustomRolePermission(
    team.id,
    "team:domain:view",
    TEAM_RESOURCE,
    true,
  );
  const canCreateCustom = useHasCustomRolePermission(
    team.id,
    "team:domain:create",
    TEAM_RESOURCE,
    false,
  );
  const canDeleteCustom = useHasCustomRolePermission(
    team.id,
    "team:domain:delete",
    TEAM_RESOURCE,
    false,
  );
  const canCreate = isTeamAdmin || canCreateCustom === true;
  const canDelete = isTeamAdmin || canDeleteCustom === true;

  const entitlements = useTeamEntitlements(team.id);
  // The portal link is entitled by either product the member can reach.
  const entitled =
    (entitlements?.ssoEnabled ?? false) ||
    (directorySync && (entitlements?.directorySyncEnabled ?? false));
  const { data: domains } = useTeamDomains(team.id, {
    isPaused: canView !== true,
  });
  const profileEmails = useProfileEmails();
  // Without a verified email on one of these domains the member can't complete
  // an SSO login through any of them. Stays false until the emails load, so
  // the warning doesn't flash on for members who do have one, and until there
  // is a domain for it to be about.
  const missingVerifiedEmail =
    profileEmails !== undefined &&
    domains !== undefined &&
    domains.length > 0 &&
    !profileEmails.some(
      (email) =>
        email.isVerified &&
        domains.some(
          (d) =>
            d.domain.toLowerCase() === email.email.split("@")[1]?.toLowerCase(),
        ),
    );
  const generatePortalLink = useDomainPortalLink(team.id);
  const [isGeneratingLink, setIsGeneratingLink] = useState(false);

  if (canView === false) {
    return (
      <SettingsSheet
        title="Authentication Domains"
        description={copy.description}
        descriptionTip={copy.descriptionTip}
      >
        <div className="px-6 py-10">
          <NoPermissionMessage
            message="You do not have permission to view domains."
            missingPermission="team:domain:view"
          />
        </div>
      </SettingsSheet>
    );
  }

  const addDomainButton = (
    <Button
      size="sm"
      className="w-fit shrink-0"
      icon={<ExternalLinkIcon />}
      loading={isGeneratingLink}
      disabled={!canCreate || !entitled || isGeneratingLink}
      tip={
        !canCreate
          ? permissionDeniedTip(
              "You do not have permission to add domains.",
              "team:domain:create",
            )
          : !entitled
            ? copy.unentitled
            : undefined
      }
      onClick={async () => {
        setIsGeneratingLink(true);
        try {
          const result = await generatePortalLink();
          if (result?.link) {
            window.open(result.link, "_blank");
          }
        } finally {
          setIsGeneratingLink(false);
        }
      }}
    >
      Add a domain
    </Button>
  );

  return (
    <SettingsSheet
      title="Authentication Domains"
      description={copy.description}
      descriptionTip={copy.descriptionTip}
      // One warning for the team's domains rather than one per row: a member
      // needs a verified email on any one of them, not on each.
      badge={
        missingVerifiedEmail && (
          <Tooltip
            tip={
              <div className="flex flex-col gap-1">
                <span>
                  None of the verified emails on your Convex account are on
                  these domains. Until one of them is, you will not be able to
                  log in with SSO.
                </span>
                <Link href="/profile">
                  You may verify an email on the profile page.
                </Link>
              </div>
            }
            side="right"
            aria-label="No verified email on these domains"
          >
            <span className="flex w-fit items-center rounded-full bg-background-warning p-1 text-content-warning">
              <ExclamationTriangleIcon className="size-3" />
            </span>
          </Tooltip>
        )
      }
      // With domains listed, adding one acts on the whole list rather than on
      // any row, so the control sits in the header. The empty state carries
      // its own copy of it instead.
      action={domains && domains.length > 0 ? addDomainButton : undefined}
    >
      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "m-3 h-10" }}
      >
        {/* The query is paused until the member's roles arrive, and SWR only
            reports `isLoading` on the mount that starts a request, so absent
            domains — not `isLoading` — is what says this is still loading. */}
        {domains &&
          (domains.length > 0 ? (
            <table className="w-full border-collapse">
              <thead>
                <tr className="border-b">
                  {/* The domain column absorbs the slack, so status sits
                      against its own column rather than in the middle of the
                      row. */}
                  <th className={cn(HEADER_CELL, "w-full")}>Domain</th>
                  <th className={HEADER_CELL}>Status</th>
                  <th className={cn(HEADER_CELL, "w-0")}>
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {domains.map((domain) => (
                  <DomainRow
                    key={domain.id}
                    teamId={team.id}
                    domain={domain}
                    canDelete={canDelete}
                  />
                ))}
              </tbody>
            </table>
          ) : (
            <EmptyStateRow message={copy.empty} action={addDomainButton} />
          ))}
      </LoadingTransition>
    </SettingsSheet>
  );
}

// The cells reach the sheet's edges, so they carry its inset themselves.
const CELL = "px-4 py-3 align-middle";
const HEADER_CELL =
  "px-4 py-2 text-left text-sm font-normal text-content-secondary";

function DomainRow({
  teamId,
  domain,
  canDelete,
}: {
  teamId: number;
  domain: SsoOrganizationDomain;
  canDelete: boolean;
}) {
  const deleteDomain = useDeleteTeamDomain(teamId, domain.id);
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [deleteError, setDeleteError] = useState<string>();
  const [isDeleting, setIsDeleting] = useState(false);

  return (
    <tr className="border-b last:border-b-0">
      <td className={cn(CELL, "truncate")}>{domain.domain}</td>
      <td className={CELL}>
        <DomainStatusBadge state={domain.state} />
      </td>
      <td className={cn(CELL, "text-right")}>
        <Menu
          placement="bottom-end"
          buttonProps={{
            variant: "neutral",
            size: "xs",
            icon: <DotsVerticalIcon />,
            "aria-label": `${domain.domain} options`,
          }}
        >
          <MenuItem
            variant="danger"
            disabled={!canDelete}
            tip={
              canDelete
                ? undefined
                : permissionDeniedTip(
                    "You do not have permission to delete domains.",
                    "team:domain:delete",
                  )
            }
            action={() => setShowDeleteConfirmation(true)}
          >
            Delete domain
          </MenuItem>
        </Menu>
        {showDeleteConfirmation && (
          <ConfirmationDialog
            onClose={() => {
              if (isDeleting) {
                return;
              }
              setShowDeleteConfirmation(false);
              setDeleteError(undefined);
            }}
            onConfirm={async () => {
              setIsDeleting(true);
              try {
                await deleteDomain();
                setShowDeleteConfirmation(false);
              } catch (e: any) {
                setDeleteError(e.message);
                throw e;
              } finally {
                setIsDeleting(false);
              }
            }}
            confirmText="Delete"
            variant="danger"
            dialogTitle="Delete domain"
            dialogBody={`Team members with an email on ${domain.domain} will no longer be managed by SSO or Directory Sync. You can add the domain again, but it will have to be verified from scratch.`}
            error={deleteError}
            validationText={domain.domain}
          />
        )}
      </td>
    </tr>
  );
}
