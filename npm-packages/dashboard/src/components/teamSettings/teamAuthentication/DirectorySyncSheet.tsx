import { useState } from "react";
import { DotsVerticalIcon, ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Loading } from "@ui/Loading";
import { Menu, MenuItem } from "@ui/Menu";
import { cn } from "@ui/cn";
import type { TeamResponse } from "generatedApi";
import {
  useDisableDirectorySync,
  useGenerateDirectorySyncConfigurationLink,
  useGetDirectorySync,
} from "api/directorySync";
import {
  useHasCustomRolePermission,
  useIsCurrentMemberTeamAdmin,
} from "api/roles";
import { useGetSSO, useTeamEntitlements } from "api/teams";
import { NoPermissionMessage } from "elements/NoPermissionMessage";
import { permissionDeniedTip } from "elements/permissionDeniedTip";
import {
  DIRECTORY_SYNC_RESOURCE,
  MEMBER_RESOURCE,
  SSO_RESOURCE,
} from "lib/permissions";
import { ConnectDirectoryDialog } from "./ConnectDirectoryDialog";
import { DirectoryGroupsSheet } from "./DirectoryGroupsSheet";
import { ProviderIcon } from "./ProviderIcon";
import { ReviewDirectoryChangesModal } from "./ReviewDirectoryChangesModal";
import {
  ConfigurationRow,
  EmptyStateRow,
  SettingsSheet,
  SHEET_ROW,
} from "./SettingsSheet";
import { DirectoryStatusBadge } from "./StatusBadge";
import { directoryProvider, useReportUnmappedProviders } from "./providers";

const NOT_CONFIGURED_DESCRIPTION =
  "Configure Directory Sync to automatically provision and deprovision team members.";

export function DirectorySyncSheet({ team }: { team: TeamResponse }) {
  const isTeamAdmin = useIsCurrentMemberTeamAdmin();
  const canView = useHasCustomRolePermission(
    team.id,
    "directorySync:view",
    DIRECTORY_SYNC_RESOURCE,
    true,
  );
  const canEnableCustom = useHasCustomRolePermission(
    team.id,
    "directorySync:enable",
    DIRECTORY_SYNC_RESOURCE,
    false,
  );
  const canDisableCustom = useHasCustomRolePermission(
    team.id,
    "directorySync:disable",
    DIRECTORY_SYNC_RESOURCE,
    false,
  );
  const canUpdateMappingCustom = useHasCustomRolePermission(
    team.id,
    "directorySync:updateGroupMapping",
    DIRECTORY_SYNC_RESOURCE,
    false,
  );
  // The staged roster names each member and the role they hold, so the
  // endpoint behind the review modal asks for `member:view` as well.
  const canViewMembers = useHasCustomRolePermission(
    team.id,
    "member:view",
    MEMBER_RESOURCE,
    true,
  );
  const canEnable = isTeamAdmin || canEnableCustom === true;
  const canDisable = isTeamAdmin || canDisableCustom === true;
  const canUpdateMapping = isTeamAdmin || canUpdateMappingCustom === true;

  const entitlements = useTeamEntitlements(team.id);
  const directorySyncEntitled = entitlements?.directorySyncEnabled ?? false;
  const customRolesEnabled = entitlements?.customRolesEnabled ?? false;
  const { data: directorySync } = useGetDirectorySync(team.id, {
    isPaused: canView !== true,
  });
  // A directory can only provision members on a verified domain, so the
  // domains sheet gates this one too.
  const canViewSSO = useHasCustomRolePermission(
    team.id,
    "sso:view",
    SSO_RESOURCE,
    true,
  );
  const { data: sso } = useGetSSO(team.id, { isPaused: canViewSSO !== true });
  const generateLink = useGenerateDirectorySyncConfigurationLink(team.id);
  const disableDirectorySync = useDisableDirectorySync(team.id);

  const provider = directoryProvider(directorySync?.directory?.type);
  useReportUnmappedProviders([provider]);

  const [isGeneratingLink, setIsGeneratingLink] = useState(false);
  const [showConnectDialog, setShowConnectDialog] = useState(false);
  const [showDisableConfirmation, setShowDisableConfirmation] = useState(false);
  const [isDisabling, setIsDisabling] = useState(false);
  const [disableError, setDisableError] = useState<string>();
  const [showReview, setShowReview] = useState(false);

  if (canView === false) {
    return (
      <SettingsSheet
        title="Directory sync"
        description={NOT_CONFIGURED_DESCRIPTION}
      >
        <div className="px-6 py-10">
          <NoPermissionMessage
            message="You do not have permission to view Directory Sync settings."
            missingPermission="directorySync:view"
          />
        </div>
      </SettingsSheet>
    );
  }

  const reported = directorySync?.directory ?? undefined;
  const directory = reported?.state === "deleting" ? undefined : reported;
  // The query is paused until the member's roles resolve `canView`, and SWR
  // only reports `isLoading` on the mount that starts a request — a query that
  // unpauses later reports `false` with nothing to show. Absent data is what
  // says this is still loading; otherwise the sheet renders its not-configured
  // state, Configure button and all, before the skeleton appears.
  const isLoadingDirectory = directorySync === undefined;
  const hasVerifiedDomain = (sso?.domains ?? []).some(
    (d) => d.state === "verified" || d.state === "legacyVerified",
  );

  const directoryTitle = provider?.label ?? directory?.name ?? "Directory";

  const openPortal = async () => {
    setIsGeneratingLink(true);
    try {
      const result = await generateLink();
      if (result?.link) {
        window.open(result.link, "_blank");
      }
    } finally {
      setIsGeneratingLink(false);
    }
  };

  const configureTip = !canEnable
    ? permissionDeniedTip(
        "You do not have permission to configure Directory Sync.",
        "directorySync:enable",
      )
    : !directorySyncEntitled
      ? // The button carries the upsell, so an unentitled team reads why it
        // cannot configure directory sync where it would have configured it.
        "Directory Sync is not available on your plan. Upgrade your plan to use Directory Sync."
      : !hasVerifiedDomain
        ? "Verify a domain before configuring Directory Sync."
        : undefined;
  const canOpenPortal =
    canEnable &&
    directorySyncEntitled &&
    hasVerifiedDomain &&
    !isGeneratingLink;

  const managementEnabled = directorySync?.enabled ?? false;
  const rosterTip =
    canViewMembers === false
      ? permissionDeniedTip(
          "You do not have permission to view team members.",
          "member:view",
        )
      : undefined;
  const reviewTip =
    rosterTip ??
    (!canEnable
      ? permissionDeniedTip(
          "You do not have permission to enable directory sync.",
          "directorySync:enable",
        )
      : !directorySyncEntitled
        ? "Directory Sync is not available on your plan."
        : undefined);

  return (
    // The groups sheet reads as part of the directory's configuration, so the
    // two sit closer together than the page's sections do.
    <div className="flex flex-col gap-6">
      <SettingsSheet
        title="Directory sync"
        description={
          directory
            ? "Manage your Directory Sync configuration."
            : NOT_CONFIGURED_DESCRIPTION
        }
      >
        {isLoadingDirectory ? (
          <Loading fullHeight={false} className="m-3 h-10" />
        ) : directory ? (
          <>
            <ConfigurationRow
              title={directoryTitle}
              icon={provider && <ProviderIcon provider={provider} />}
              badge={<DirectoryStatusBadge state={directory.state} />}
              menu={
                <Menu
                  placement="bottom-end"
                  buttonProps={{
                    variant: "neutral",
                    size: "xs",
                    icon: <DotsVerticalIcon />,
                    "aria-label": `${directoryTitle} options`,
                  }}
                >
                  {managementEnabled ? (
                    <MenuItem
                      disabled={rosterTip !== undefined}
                      tip={rosterTip}
                      action={() => setShowReview(true)}
                    >
                      View pending members
                    </MenuItem>
                  ) : null}
                  <MenuItem
                    disabled={!canOpenPortal}
                    tip={configureTip}
                    action={() => {
                      void openPortal();
                    }}
                  >
                    Manage
                  </MenuItem>
                  <MenuItem
                    variant="danger"
                    disabled={!canDisable}
                    tip={
                      canDisable
                        ? undefined
                        : permissionDeniedTip(
                            "You do not have permission to disable Directory Sync.",
                            "directorySync:disable",
                          )
                    }
                    action={() => setShowDisableConfirmation(true)}
                  >
                    Disable Directory Sync
                  </MenuItem>
                </Menu>
              }
            />
            {!directory.linked ? (
              <div
                className={cn(
                  SHEET_ROW,
                  "flex items-center gap-4 border-t text-sm",
                )}
              >
                <span className="text-content-secondary">
                  This directory is not linked yet. Check its connection with
                  your identity provider to finish setting up directory sync.
                </span>
                <div className="ml-auto">
                  <Button
                    size="xs"
                    icon={<ExternalLinkIcon />}
                    disabled={!canOpenPortal}
                    tip={configureTip}
                    onClick={() => setShowConnectDialog(true)}
                  >
                    Check connection
                  </Button>
                </div>
              </div>
            ) : !managementEnabled ? (
              <div
                className={cn(
                  SHEET_ROW,
                  "flex items-center gap-4 border-t text-sm",
                )}
              >
                <span className="text-content-secondary">
                  Directory sync is not yet enabled. Please review directory
                  role mappings to enable automatic provisioning.
                </span>
                <div className="ml-auto">
                  <Button
                    size="xs"
                    disabled={reviewTip !== undefined}
                    tip={reviewTip}
                    onClick={() => setShowReview(true)}
                  >
                    Review
                  </Button>
                </div>
              </div>
            ) : null}
            {showReview && (
              <ReviewDirectoryChangesModal
                team={team}
                enabled={managementEnabled}
                onClose={() => setShowReview(false)}
              />
            )}
          </>
        ) : (
          <EmptyStateRow
            message="Directory Sync has not been configured."
            action={
              <Button
                size="sm"
                className="w-fit shrink-0"
                icon={<ExternalLinkIcon />}
                disabled={!canOpenPortal}
                tip={configureTip}
                onClick={() => setShowConnectDialog(true)}
              >
                Configure
              </Button>
            }
          />
        )}

        {showConnectDialog && (
          <ConnectDirectoryDialog
            teamId={team.id}
            onClose={() => setShowConnectDialog(false)}
          />
        )}

        {showDisableConfirmation && (
          <ConfirmationDialog
            onClose={() => {
              if (isDisabling) {
                return;
              }
              setShowDisableConfirmation(false);
              setDisableError(undefined);
            }}
            onConfirm={async () => {
              setIsDisabling(true);
              try {
                await disableDirectorySync();
                setShowDisableConfirmation(false);
              } catch (e: any) {
                setDisableError(e.message);
                throw e;
              } finally {
                setIsDisabling(false);
              }
            }}
            confirmText="Disable"
            variant="danger"
            dialogTitle="Disable Directory Sync"
            dialogBody="This disconnects your directory, and team members will no longer be provisioned or deprovisioned by your identity provider. Members already on the team keep their access."
            error={disableError}
            validationText="DISABLE DIRECTORY SYNC"
          />
        )}
      </SettingsSheet>
      {directory?.linked && (
        <DirectoryGroupsSheet
          team={team}
          canEdit={canUpdateMapping}
          customRolesEnabled={customRolesEnabled}
        />
      )}
    </div>
  );
}
