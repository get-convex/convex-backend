import type { FC } from "react";
import {
  ArchiveIcon,
  DownloadIcon,
  GearIcon,
  GlobeIcon,
  Link2Icon,
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
};
