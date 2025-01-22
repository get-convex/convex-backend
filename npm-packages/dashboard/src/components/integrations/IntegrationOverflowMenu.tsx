import { DotsVerticalIcon, PlusIcon } from "@radix-ui/react-icons";
import {
  Button,
  configToUrl,
  ExceptionReportingIntegration,
  integrationName,
  LogIntegration,
} from "dashboard-common";
import { Menu, MenuItem } from "elements/Menu";
import { useDeleteSink } from "hooks/deploymentApi";
import { useState } from "react";
import { ConfirmationDialog } from "elements/ConfirmationDialog";

export function IntegrationOverflowMenu({
  integration,
  onConfigure,
}: {
  integration: LogIntegration | ExceptionReportingIntegration;
  onConfigure: () => void;
}) {
  const deleteSink = useDeleteSink();
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  return integration.existing ? (
    <>
      {showDeleteConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDeleteConfirmation(false);
          }}
          onConfirm={() => deleteSink(integration.kind)}
          dialogTitle={`Delete ${integrationName(integration.existing.config.type)} Integration`}
          dialogBody="Are you sure you want to delete this integration?"
          confirmText="Delete"
        />
      )}
      <Menu
        placement="bottom-end"
        buttonProps={{
          "aria-label": "Open table settings",
          className: "ml-auto",
          icon: <DotsVerticalIcon />,
          size: "xs",
          variant: "neutral",
        }}
      >
        <MenuItem action={onConfigure}>Configure</MenuItem>
        <MenuItem href={configToUrl(integration.existing.config)}>
          Go to {integrationName(integration.existing.config.type)}
        </MenuItem>
        <MenuItem
          action={() => {
            setShowDeleteConfirmation(true);
          }}
          variant="danger"
        >
          Delete
        </MenuItem>
      </Menu>
    </>
  ) : (
    <Button
      size="xs"
      icon={<PlusIcon />}
      variant="neutral"
      tip="Configure Integration"
      tipSide="right"
      onClick={onConfigure}
    />
  );
}
