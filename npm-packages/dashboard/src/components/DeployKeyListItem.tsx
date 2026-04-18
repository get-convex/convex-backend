import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Checkbox } from "@ui/Checkbox";
import { Modal } from "@ui/Modal";
import { Menu, MenuItem } from "@ui/Menu";
import { HelpTooltip } from "@ui/HelpTooltip";
import { PlatformDeployKeyResponse } from "@convex-dev/platform/managementApi";
import { useCurrentTeam, useTeamMembers } from "api/teams";
import { useState } from "react";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { usePostHog } from "hooks/usePostHog";
import {
  OPERATION_GROUPS,
  formatOperationName,
} from "components/deploymentSettings/GenerateDeployKeyButton";

export function DeployKeyListItem({
  deployKey,
  deploymentType,
  onDelete,
}: {
  deployKey: PlatformDeployKeyResponse;
  deploymentType: string;
  onDelete: (args: { id: string }) => Promise<unknown>;
}) {
  const team = useCurrentTeam();
  const members = useTeamMembers(team?.id);
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [showOperations, setShowOperations] = useState(false);
  const { capture } = usePostHog();

  const member = members?.find((m) => m.id === deployKey.creator);

  // @ts-expect-error allowedOperations is not in the public API spec yet
  const allowedOperations: string[] | undefined = deployKey.allowedOperations;

  const knownOperationKeys = new Set(
    OPERATION_GROUPS.flatMap((g) => g.operations.map((op) => op.key)),
  );
  const unknownOperations = allowedOperations?.filter(
    (op) => !knownOperationKeys.has(op),
  );

  return (
    <div key={deployKey.name} className="flex w-full flex-col">
      <div className="my-2 flex flex-wrap items-center justify-between gap-2">
        <div>
          {deployKey.name || (
            <span className="text-content-secondary italic">Unnamed</span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex flex-col items-end">
            {deployKey.lastUsedTime !== null &&
            deployKey.lastUsedTime !== undefined ? (
              <TimestampDistance
                prefix="Last used "
                date={new Date(deployKey.lastUsedTime)}
              />
            ) : (
              <div className="text-xs text-content-secondary">Never used</div>
            )}
            <div className="flex gap-1">
              <TimestampDistance
                prefix="Created "
                date={new Date(deployKey.creationTime)}
              />
              <div className="flex items-center gap-1 text-xs text-content-secondary">
                by{" "}
                {member ? (
                  <TeamMemberLink
                    memberId={deployKey.creator}
                    name={member?.name || member?.email}
                  />
                ) : (
                  "Unknown member"
                )}
              </div>
            </div>
          </div>
          <Menu
            placement="bottom-end"
            buttonProps={{
              variant: "neutral",
              size: "xs",
              icon: <DotsVerticalIcon />,
              "aria-label": "Deploy key options",
            }}
          >
            {allowedOperations !== undefined ? (
              <MenuItem action={() => setShowOperations(true)}>
                View allowed operations
              </MenuItem>
            ) : null}
            <MenuItem
              variant="danger"
              action={() => setShowDeleteConfirmation(true)}
            >
              Delete
            </MenuItem>
          </Menu>
        </div>
      </div>
      {showDeleteConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDeleteConfirmation(false);
          }}
          onConfirm={async () => {
            await onDelete({ id: deployKey.name });
            capture("deleted_deploy_key", {
              type: deploymentType,
            });
          }}
          confirmText="Delete"
          dialogTitle="Delete Deploy Key"
          dialogBody={
            <>
              Are you sure you want to delete:{" "}
              <span className="font-semibold">{deployKey.name}</span>?
            </>
          }
        />
      )}
      {showOperations && allowedOperations !== undefined && (
        <Modal
          title="Allowed Operations"
          onClose={() => setShowOperations(false)}
          size="md"
        >
          <div className="scrollbar max-h-[60dvh] overflow-y-auto">
            <div className="flex flex-col gap-3">
              {OPERATION_GROUPS.map((group) => {
                const groupOps = group.operations.filter((op) =>
                  allowedOperations.includes(op.key),
                );
                if (groupOps.length === 0) return null;
                return (
                  <div key={group.label}>
                    <div className="mb-1 text-sm font-semibold text-content-secondary">
                      {group.label}
                    </div>
                    <div className="grid grid-cols-[repeat(auto-fill,minmax(12rem,1fr))] gap-x-4 gap-y-1">
                      {groupOps.map((op) => (
                        <label
                          key={op.key}
                          className="flex items-center gap-2 rounded px-1 py-1 text-xs"
                        >
                          <Checkbox checked disabled onChange={() => {}} />
                          {op.label}
                          <HelpTooltip>{op.description}</HelpTooltip>
                        </label>
                      ))}
                    </div>
                  </div>
                );
              })}
              {unknownOperations && unknownOperations.length > 0 && (
                <div>
                  <div className="mb-1 text-sm font-semibold text-content-secondary">
                    Other
                  </div>
                  <div className="grid grid-cols-[repeat(auto-fill,minmax(12rem,1fr))] gap-x-4 gap-y-1">
                    {unknownOperations.map((op) => (
                      <label
                        key={op}
                        className="flex items-center gap-2 rounded px-1 py-1 text-xs"
                      >
                        <Checkbox checked disabled onChange={() => {}} />
                        {formatOperationName(op)}
                      </label>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
