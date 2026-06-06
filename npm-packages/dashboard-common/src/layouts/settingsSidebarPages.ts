export const DEPLOYMENT_SETTINGS_PAGES_AND_NAMES = {
  general: "General",
  "environment-variables": "Environment Variables",
  authentication: "Authentication",
  "custom-domains": "Custom Domains",
  snapshots: "Snapshot Import & Export",
  components: "Components",
  backups: "Backup & Restore",
  integrations: "Integrations",
  "admin-keys": "Admin Keys",
};

export type SettingsPageKind = keyof typeof DEPLOYMENT_SETTINGS_PAGES_AND_NAMES;

const DEPLOYMENT_SETTINGS_PAGES = Object.keys(
  DEPLOYMENT_SETTINGS_PAGES_AND_NAMES,
) as SettingsPageKind[];

export function getAllowedDeploymentSettingsPages({
  nents,
  showAdminKeys,
}: {
  nents?: readonly unknown[];
  showAdminKeys: boolean;
}) {
  let pages = DEPLOYMENT_SETTINGS_PAGES;

  if (nents?.length === 0) {
    pages = pages.filter((d) => d !== "components");
  }

  pages = pages.filter((d) => d !== "snapshots");

  // Admin Keys management is shown for backends that own that surface -
  // single-deployment self-hosted shells and orchestrator-managed deployments.
  if (!showAdminKeys) {
    pages = pages.filter((d) => d !== "admin-keys");
  }

  return pages;
}
