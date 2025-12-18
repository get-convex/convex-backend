import { DotsVerticalIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Menu, MenuItem } from "@ui/Menu";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { useDeleteLogStream } from "@common/lib/integrationsApi";
import { useState } from "react";
import {
  LogIntegration,
  ExceptionReportingIntegration,
  integrationName,
  configToUrl,
} from "@common/lib/integrationHelpers";

export function IntegrationOverflowMenu({
  integration,
  onConfigure,
}: {
  integration: LogIntegration | ExceptionReportingIntegration;
  onConfigure: () => void;
}) {
  const deleteLogStream = useDeleteLogStream();
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const logStreamId = integration.existing?._id;

  return integration.existing && logStreamId ? (
    <>
      {showDeleteConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDeleteConfirmation(false);
          }}
          onConfirm={() => deleteLogStream(logStreamId)}
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
