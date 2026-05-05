import { DotsVerticalIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Menu, MenuItem } from "@ui/Menu";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import {
  useDeleteLogStream,
  useUpdateLogStream,
} from "@common/lib/integrationsApi";
import { toast } from "@common/lib/utils";
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
  const updateLogStream = useUpdateLogStream();
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const logStreamId = integration.existing?._id;
  const existingIntegration = integration.existing;
  const webhookConfig =
    integration.kind === "webhook"
      ? (integration.existing?.config ?? null)
      : null;

  return existingIntegration && logStreamId ? (
    <>
      {showDeleteConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDeleteConfirmation(false);
          }}
          onConfirm={async () => {
            await deleteLogStream(logStreamId);
            toast(
              "success",
              `Deleted ${integrationName(existingIntegration.config.type)} integration`,
            );
          }}
          dialogTitle={`Delete ${integrationName(existingIntegration.config.type)} Integration`}
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
        <MenuItem href={configToUrl(existingIntegration.config)}>
          Go to {integrationName(existingIntegration.config.type)}
        </MenuItem>
        {webhookConfig && (
          <MenuItem
            action={async () => {
              await updateLogStream(logStreamId, {
                logStreamType: "webhook",
                url: webhookConfig.url,
                format: webhookConfig.format,
              });
              toast("success", "Refreshed webhook connection");
            }}
          >
            Refresh connection
          </MenuItem>
        )}
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
      data-testid="configure-integration"
    />
  );
}
