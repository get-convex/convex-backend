import { useState } from "react";
import { DotsVerticalIcon, ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Loading } from "@ui/Loading";
import { Menu, MenuItem } from "@ui/Menu";
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
import { DIRECTORY_SYNC_RESOURCE, SSO_RESOURCE } from "lib/permissions";
import { DirectoryGroupsSheet } from "./DirectoryGroupsSheet";
import {
  ConfigurationRow,
  EmptyStateRow,
  SettingsSheet,
} from "./SettingsSheet";
import { DirectoryStatusBadge } from "./StatusBadge";

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

  const [isGeneratingLink, setIsGeneratingLink] = useState(false);
  const [showDisableConfirmation, setShowDisableConfirmation] = useState(false);
  const [isDisabling, setIsDisabling] = useState(false);
  const [disableError, setDisableError] = useState<string>();

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

  const directory = directorySync?.directory ?? undefined;
  // The query is paused until the member's roles resolve `canView`, and SWR
  // only reports `isLoading` on the mount that starts a request — a query that
  // unpauses later reports `false` with nothing to show. Absent data is what
  // says this is still loading; otherwise the sheet renders its not-configured
  // state, Configure button and all, before the skeleton appears.
  const isLoadingDirectory = directorySync === undefined;
  const hasVerifiedDomain = (sso?.domains ?? []).some(
    (d) => d.state === "verified" || d.state === "legacyVerified",
  );

  // WorkOS's own directory name is whatever the admin typed on the IdP side,
  // so the type it reports ("okta scim v2.0") identifies the directory more
  // reliably, and matches the connection type on the SSO row above.
  const directoryTitle = directory?.type ?? directory?.name ?? "Directory";

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
          <ConfigurationRow
            title={directoryTitle}
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
        ) : (
          <EmptyStateRow
            message="Directory Sync has not been configured."
            action={
              <Button
                size="sm"
                className="w-fit shrink-0"
                icon={<ExternalLinkIcon />}
                loading={isGeneratingLink}
                disabled={!canOpenPortal}
                tip={configureTip}
                onClick={openPortal}
              >
                Configure
              </Button>
            }
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
      {directory && (
        <DirectoryGroupsSheet
          team={team}
          canEdit={canUpdateMapping}
          customRolesEnabled={customRolesEnabled}
        />
      )}
    </div>
  );
}
