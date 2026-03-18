import { useFlags } from "launchdarkly-react-client-sdk";
import kebabCase from "lodash/kebabCase";

export const flagDefaults: {
  commandPalette: boolean;
  commandPaletteDeleteProjects: boolean;
  singleSignOn: boolean;
  workOsEnvironmentProvisioningDashboardUi: boolean;
  enableNewDashboardVersionNotification: boolean;
  enableStatuspageWidget: boolean;
  connectionStateCheckIntervalMs: number;
  showReferences: boolean;
  deploymentList: boolean;
  postHogIntegrations: boolean;
  usageDashboardV2: boolean;
  transferDeployment: boolean;
} = {
  commandPalette: false,
  commandPaletteDeleteProjects: false,
  singleSignOn: false,
  workOsEnvironmentProvisioningDashboardUi: false,
  enableNewDashboardVersionNotification: false,
  enableStatuspageWidget: true,
  connectionStateCheckIntervalMs: 2500,
  showReferences: false,
  deploymentList: false,
  postHogIntegrations: false,
  usageDashboardV2: false,
  transferDeployment: false,
};

// Flag defaults need to be in the default kebab-case format:
// https://docs.launchdarkly.com/sdk/client-side/react/react-web#configuring-the-react-sdk
// Note: kebabCaseKeys uses lodash kebabCase which splits "V2" into "v-2".
// We fix keys where this produces incorrect results.
const KEBAB_CASE_OVERRIDES: Record<string, string> = {
  usageDashboardV2: "usage-dashboard-v2",
};

function kebabCaseKey(key: string): string {
  return KEBAB_CASE_OVERRIDES[key] ?? kebabCase(key);
}

export const flagDefaultsKebabCase = Object.entries(flagDefaults).reduce(
  (carry, [key, value]) => ({ ...carry, [kebabCaseKey(key)]: value }),
  {} as { [key: string]: any },
);

// useLaunchDarkly is a thin wrapper on LaunchDarkly's react sdk which adds manual to flag keys.
// At some point, we can generate this file.
export function useLaunchDarkly() {
  return useFlags<typeof flagDefaults>();
}
