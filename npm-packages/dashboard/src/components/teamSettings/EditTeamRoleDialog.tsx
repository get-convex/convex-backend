import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import { Link } from "@ui/Link";
import { Menu, MenuItem } from "@ui/Menu";
import { Modal } from "@ui/Modal";
import { Tooltip } from "@ui/Tooltip";
import { Cross2Icon, PlusIcon } from "@radix-ui/react-icons";
import { useMemo, useState } from "react";
import sortBy from "lodash/sortBy";
import type { CustomRoleResponse } from "@convex-dev/platform/managementApi";
import type { TeamMember, TeamResponse } from "generatedApi";

type RoleChoice = "admin" | "developer" | "custom";

function sameIds(a: number[], b: number[]) {
  if (a.length !== b.length) return false;
  const aSorted = [...a].sort((x, y) => x - y);
  const bSorted = [...b].sort((x, y) => x - y);
  return aSorted.every((v, i) => v === bSorted[i]);
}

export function EditTeamRoleDialog({
  team,
  member,
  customRoles,
  customRolesEnabled,
  customRolesVisible,
  onSave,
  onClose,
}: {
  team: TeamResponse;
  member: TeamMember;
  customRoles: CustomRoleResponse[];
  customRolesEnabled: boolean;
  customRolesVisible: boolean;
  onSave: (body: {
    memberId: number;
    role?: "admin" | "developer";
    customRoles?: number[];
  }) => Promise<unknown>;
  onClose: () => void;
}) {
  const memberCustomRoleIds = (member.customRoles ?? []).map((c) => c.id);
  const [choice, setChoice] = useState<RoleChoice>(
    member.role === "custom" ? "custom" : member.role,
  );
  const [selectedCustomRoleIds, setSelectedCustomRoleIds] =
    useState<number[]>(memberCustomRoleIds);
  const [isSaving, setIsSaving] = useState(false);
  const [didAttemptSave, setDidAttemptSave] = useState(false);

  const sortedCustomRoles = useMemo(
    () => sortBy(customRoles, (r) => r.name.toLowerCase()),
    [customRoles],
  );
  const customRoleNameById = useMemo(
    () => new Map(sortedCustomRoles.map((r) => [r.id, r.name] as const)),
    [sortedCustomRoles],
  );
  const selectedSet = useMemo(
    () => new Set(selectedCustomRoleIds),
    [selectedCustomRoleIds],
  );
  const selectedSorted = useMemo(
    () =>
      sortBy(selectedCustomRoleIds, (id) =>
        (customRoleNameById.get(id) ?? "").toLowerCase(),
      ),
    [selectedCustomRoleIds, customRoleNameById],
  );
  const unselectedRoles = useMemo(
    () => sortedCustomRoles.filter((r) => !selectedSet.has(r.id)),
    [sortedCustomRoles, selectedSet],
  );

  const roleOptions = [
    { label: "Admin", value: "admin" as const, disabled: false },
    { label: "Developer", value: "developer" as const, disabled: false },
    ...(customRolesVisible
      ? [
          {
            label: "Custom",
            value: "custom" as const,
            disabled: !customRolesEnabled,
          },
        ]
      : []),
  ];

  const isUnchanged =
    choice === "custom"
      ? member.role === "custom" &&
        sameIds(selectedCustomRoleIds, memberCustomRoleIds)
      : member.role === choice;
  const customSelectionEmpty =
    choice === "custom" && selectedCustomRoleIds.length === 0;
  const noCustomRolesAvailable =
    choice === "custom" && sortedCustomRoles.length === 0;

  const removeCustomRole = (id: number) =>
    setSelectedCustomRoleIds((ids) => ids.filter((x) => x !== id));
  const addCustomRole = (id: number) =>
    setSelectedCustomRoleIds((ids) => (ids.includes(id) ? ids : [...ids, id]));

  return (
    <Modal
      title="Edit team role"
      description={`Change the team role for ${member.name || member.email}.`}
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
            if (choice === "custom") {
              await onSave({
                memberId: member.id,
                customRoles: selectedCustomRoleIds,
              });
            } else {
              await onSave({ memberId: member.id, role: choice });
            }
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
            Option={({ label, disabled }) =>
              disabled ? (
                <Tooltip
                  tip="Custom roles are not enabled for this team."
                  side="left"
                >
                  <span>{label}</span>
                </Tooltip>
              ) : (
                <span>{label}</span>
              )
            }
          />
        </div>

        {choice === "custom" && (
          <div className="flex flex-col gap-1">
            <span className="text-sm text-content-primary">Custom roles</span>
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
            <div className="flex flex-wrap items-center gap-1.5">
              {selectedSorted.map((id) => {
                const name = customRoleNameById.get(id) ?? `Role #${id}`;
                return (
                  <Button
                    key={id}
                    variant="unstyled"
                    aria-label={`Remove custom role ${name}`}
                    tip="Remove custom role"
                    onClick={() => removeCustomRole(id)}
                    className="flex cursor-pointer items-center gap-1 rounded-sm border px-1.5 py-1 text-xs text-content-primary shadow-xs transition-colors hover:border-content-error hover:bg-background-error hover:text-content-error hover:shadow-none"
                  >
                    <span className="max-w-[12rem] truncate">{name}</span>
                    <Cross2Icon className="h-3 w-3 shrink-0" />
                  </Button>
                );
              })}
              {unselectedRoles.length === 0 ? (
                <Button
                  variant="neutral"
                  size="xs"
                  icon={<PlusIcon />}
                  aria-label="Add custom role"
                  tip={
                    noCustomRolesAvailable
                      ? "No custom roles exist for this team yet."
                      : "All custom roles are already added."
                  }
                  disabled
                />
              ) : (
                <Menu
                  placement="bottom-start"
                  buttonProps={{
                    variant: "neutral",
                    size: "xs",
                    icon: <PlusIcon />,
                    "aria-label": "Add custom role",
                  }}
                >
                  {unselectedRoles.map((r) => (
                    <MenuItem key={r.id} action={() => addCustomRole(r.id)}>
                      {r.name}
                    </MenuItem>
                  ))}
                </Menu>
              )}
            </div>
            {noCustomRolesAvailable && (
              <span className="text-xs text-content-secondary">
                No custom roles exist for this team yet.
              </span>
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
