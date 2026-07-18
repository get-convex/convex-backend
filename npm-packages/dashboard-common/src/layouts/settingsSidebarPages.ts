import type { FC } from "react";
import {
  ArchiveIcon,
  DownloadIcon,
  GearIcon,
  GlobeIcon,
  Link2Icon,
  LockClosedIcon,
} from "@radix-ui/react-icons";
import {
  HandRaisedIcon,
  KeyIcon,
  VariableIcon,
} from "@heroicons/react/24/outline";
import { PuzzlePieceIcon } from "@common/elements/icons";

export const DEPLOYMENT_SETTINGS_PAGES_AND_NAMES = {
  general: "General",
  "environment-variables": "Environment Variables",
  "usage-limits": "Usage Limits",
  authentication: "Authentication",
  "custom-domains": "Custom Domains",
  snapshots: "Snapshot Import & Export",
  components: "Components",
  backups: "Backup & Restore",
  integrations: "Integrations",
  "admin-keys": "Admin Keys",
};

export type SettingsPageKind = keyof typeof DEPLOYMENT_SETTINGS_PAGES_AND_NAMES;

// Icon shown next to each deployment settings page in the settings sidebar.
export const DEPLOYMENT_SETTINGS_PAGE_ICONS: Record<
  SettingsPageKind,
  FC<{ className?: string }>
> = {
  general: GearIcon,
  "environment-variables": VariableIcon,
  "usage-limits": HandRaisedIcon,
  authentication: KeyIcon,
  "custom-domains": GlobeIcon,
  snapshots: DownloadIcon,
  // Same icon as the NentSwitcher.
  components: PuzzlePieceIcon,
  backups: ArchiveIcon,
  integrations: Link2Icon,
  "admin-keys": LockClosedIcon,
};

const DEPLOYMENT_SETTINGS_PAGES = Object.keys(
  DEPLOYMENT_SETTINGS_PAGES_AND_NAMES,
) as SettingsPageKind[];

export function getAllowedDeploymentSettingsPages({
  nents,
  showAdminKeys,
  usageLimitsEnabled,
}: {
  nents?: readonly unknown[];
  showAdminKeys: boolean;
  usageLimitsEnabled: boolean;
}) {
  let pages = DEPLOYMENT_SETTINGS_PAGES;

  if (nents?.length === 0) {
    pages = pages.filter((d) => d !== "components");
  }

  pages = pages.filter((d) => d !== "snapshots");

  // Usage limits is feature-flagged; hide it from the sidebar when off.
  if (!usageLimitsEnabled) {
    pages = pages.filter((d) => d !== "usage-limits");
  }

  // Admin Keys management is shown for backends that own that surface -
  // single-deployment self-hosted shells and orchestrator-managed deployments.
  if (!showAdminKeys) {
    pages = pages.filter((d) => d !== "admin-keys");
  }

  return pages;
}
