import { useState } from "react";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { Link } from "@ui/Link";
import { Loading } from "@ui/Loading";
import { Modal } from "@ui/Modal";
import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import type { DirectoryGroupResponse, TeamResponse } from "generatedApi";
import {
  useDeleteGroupRoleMapping,
  useSetGroupRoleMapping,
} from "api/directorySync";
import { useHasCustomRolePermission, useListCustomRoles } from "api/roles";
import { NoPermissionMessage } from "elements/NoPermissionMessage";
import { CUSTOM_ROLE_RESOURCE } from "lib/permissions";
import { CustomRolesSelector } from "../CustomRolesSelector";

/** `noAccess` is a group with no mapping: it gives its members no place on the team. */
type RoleChoice = "admin" | "developer" | "custom" | "noAccess";

function sameIds(a: number[], b: number[]) {
  if (a.length !== b.length) return false;
  const aSorted = [...a].sort((x, y) => x - y);
  const bSorted = [...b].sort((x, y) => x - y);
  return aSorted.every((v, i) => v === bSorted[i]);
}

export function EditGroupRoleDialog({
  team,
  group,
  customRolesEnabled,
  onClose,
}: {
  team: TeamResponse;
  group: DirectoryGroupResponse;
  customRolesEnabled: boolean;
  onClose: () => void;
}) {
  // An unmapped group confers nothing, so that is what the dialog starts from.
  const currentRole: RoleChoice = group.mapping?.role ?? "noAccess";
  const currentCustomRoleIds = (group.mapping?.customRoles ?? []).map(
    (r) => r.id,
  );
  const [choice, setChoice] = useState<RoleChoice>(currentRole);
  const [selectedCustomRoleIds, setSelectedCustomRoleIds] =
    useState<number[]>(currentCustomRoleIds);
  const [isSaving, setIsSaving] = useState(false);
  const [didAttemptSave, setDidAttemptSave] = useState(false);
  const setMapping = useSetGroupRoleMapping(team.id, group.workosGroupId);
  const deleteMapping = useDeleteGroupRoleMapping(team.id, group.workosGroupId);

  const canViewCustomRoles = useHasCustomRolePermission(
    team.id,
    "customRole:view",
    CUSTOM_ROLE_RESOURCE,
    true,
  );
  const customDisabledReason = !customRolesEnabled
    ? "Custom roles are not enabled for this team."
    : canViewCustomRoles === false
      ? "You do not have permission to view custom roles."
      : undefined;
  // Only the selector below needs the team's roles, so they load with the
  // dialog rather than with the group list behind it. The group rows name
  // the roles they confer on their own.
  const { data: customRolesData } = useListCustomRoles(
    customRolesEnabled && canViewCustomRoles === true ? team.id : undefined,
  );
  const customRoles = customRolesData?.items ?? [];
  // Undefined spans both the request and the permission it waits on, so an
  // empty list means "this team has none" only once both have landed.
  const customRolesLoading =
    customDisabledReason === undefined && customRolesData === undefined;
  const roleOptions = [
    { label: "No access", value: "noAccess" as const, disabled: false },
    { label: "Admin", value: "admin" as const, disabled: false },
    { label: "Developer", value: "developer" as const, disabled: false },
    {
      label: "Custom",
      value: "custom" as const,
      disabled: customDisabledReason !== undefined,
    },
  ];

  const isUnchanged =
    choice === "custom"
      ? currentRole === "custom" &&
        sameIds(selectedCustomRoleIds, currentCustomRoleIds)
      : currentRole === choice;
  const customSelectionEmpty =
    choice === "custom" && selectedCustomRoleIds.length === 0;
  // Nothing to submit as long as the roles to choose from are unread: still
  // loading, out of reach, or none defined.
  const noCustomRolesAvailable =
    choice === "custom" &&
    (customDisabledReason !== undefined ||
      customRolesLoading ||
      customRoles.length === 0);

  const save = async () => {
    // No access is the absence of a mapping, so it is the mapping coming off.
    if (choice === "noAccess") {
      await deleteMapping();
    } else if (choice === "custom") {
      await setMapping({ customRoles: selectedCustomRoleIds });
    } else {
      await setMapping({ role: choice });
    }
  };

  return (
    <Modal
      title="Edit group role"
      description={
        <>
          Change the role that members of{" "}
          <span className="font-semibold">{group.name}</span> receive.
        </>
      }
      onClose={onClose}
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={async (e) => {
          e.preventDefault();
          setDidAttemptSave(true);
          if (
            isUnchanged ||
            customSelectionEmpty ||
            noCustomRolesAvailable ||
            isSaving
          ) {
            return;
          }
          setIsSaving(true);
          try {
            await save();
            onClose();
          } finally {
            setIsSaving(false);
          }
        }}
      >
        <div className="flex flex-col gap-1">
          <Combobox
            label="Role"
            labelHidden={false}
            options={roleOptions}
            selectedOption={choice}
            setSelectedOption={(value) => {
              if (value) setChoice(value);
            }}
            disableSearch
            // `asChild` keeps the tooltip's trigger a span: the combobox
            // renders this for the selected value too, inside its button,
            // and a nested button is not a valid interactive control.
            Option={({ label, disabled }) =>
              disabled && customDisabledReason ? (
                <Tooltip tip={customDisabledReason} side="left" asChild>
                  <span>{label}</span>
                </Tooltip>
              ) : (
                <span>{label}</span>
              )
            }
          />
        </div>

        {choice === "noAccess" && (
          <p className="text-xs text-content-secondary">
            This group will not grant access to this Convex team.
          </p>
        )}

        {choice === "custom" && (
          <div className="flex flex-col gap-1">
            <p className="mb-2 text-xs text-content-secondary">
              Custom roles let you assign granular permissions to team members.
              Manage them in{" "}
              <Link
                href={`/t/${team.slug}/settings/custom-roles`}
                className="items-center"
              >
                Team Settings → Custom Roles
              </Link>
              .
            </p>
            {!customRolesEnabled ? (
              <p className="text-xs text-content-secondary">
                Custom roles are not enabled for this team.
              </p>
            ) : canViewCustomRoles === false ? (
              <Sheet className="py-4" padding={false}>
                <NoPermissionMessage
                  message="You do not have permission to view custom roles."
                  missingPermission="customRole:view"
                  size="sm"
                />
              </Sheet>
            ) : customRolesLoading ? (
              <Loading fullHeight={false} className="h-8 w-48" />
            ) : (
              <CustomRolesSelector
                availableRoles={customRoles}
                selectedIds={selectedCustomRoleIds}
                onChange={setSelectedCustomRoleIds}
              />
            )}
            {didAttemptSave &&
              customSelectionEmpty &&
              !noCustomRolesAvailable && (
                <span className="text-xs text-content-error">
                  Select at least one custom role.
                </span>
              )}
          </div>
        )}

        <div className="mt-2 flex justify-end gap-2">
          <Button variant="neutral" onClick={onClose} disabled={isSaving}>
            Cancel
          </Button>
          <Button
            type="submit"
            loading={isSaving}
            disabled={isUnchanged || noCustomRolesAvailable}
          >
            Save
          </Button>
        </div>
      </form>
    </Modal>
  );
}
