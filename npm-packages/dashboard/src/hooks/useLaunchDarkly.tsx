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
  scopedDeployKeys: boolean;
  customRoles: boolean;
} = {
  commandPalette: false,
  commandPaletteDeleteProjects: false,
  singleSignOn: false,
  workOsEnvironmentProvisioningDashboardUi: false,
  enableNewDashboardVersionNotification: false,
  enableStatuspageWidget: true,
  connectionStateCheckIntervalMs: 2500,
  scopedDeployKeys: false,
  customRoles: false,
};

export const flagDefaultsKebabCase = Object.entries(flagDefaults).reduce(
  (carry, [key, value]) => ({ ...carry, [kebabCase(key)]: value }),
  {} as { [key: string]: any },
);

// useLaunchDarkly is a thin wrapper on LaunchDarkly's react sdk which adds manual to flag keys.
// At some point, we can generate this file.
export function useLaunchDarkly() {
  return useFlags<typeof flagDefaults>();
}
