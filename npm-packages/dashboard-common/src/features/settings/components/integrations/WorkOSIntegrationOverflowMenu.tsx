import { DotsVerticalIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Menu, MenuItem, MenuLink } from "@ui/Menu";
import { AuthIntegration } from "@common/lib/integrationHelpers";

export function WorkOSIntegrationOverflowMenu({
  integration,
  onConfigure,
}: {
  integration: AuthIntegration;
  onConfigure: () => void;
}) {
  const environmentId = integration.existing?.workosEnvironmentId;

  return integration.existing ? (
    <Menu
      placement="bottom-end"
      buttonProps={{
        "aria-label": "Open integration settings",
        className: "ml-auto",
        icon: <DotsVerticalIcon />,
        size: "xs",
        variant: "neutral",
      }}
    >
      <MenuItem action={onConfigure}>View Configuration</MenuItem>
      <MenuLink
        href={`https://dashboard.workos.com/${environmentId}/authentication`}
        target="_blank"
      >
        Go to WorkOS
      </MenuLink>
    </Menu>
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
