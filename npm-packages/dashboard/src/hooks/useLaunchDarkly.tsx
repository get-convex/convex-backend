import { useFlags } from "launchdarkly-react-client-sdk";
import kebabCase from "lodash/kebabCase";

export const flagDefaults: {
  commandPalette: boolean;
  commandPaletteDeleteProjects: boolean;
  enableNewDashboardVersionNotification: boolean;
  connectionStateCheckIntervalMs: number;
  usageLimits: boolean;
  nonDefaultTeamDevsInMainMenu: number;
  copyEnvVarNameAndValue: boolean;
} = {
  commandPalette: false,
  commandPaletteDeleteProjects: false,
  enableNewDashboardVersionNotification: false,
  connectionStateCheckIntervalMs: 2500,
  usageLimits: false,
  nonDefaultTeamDevsInMainMenu: 10,
  copyEnvVarNameAndValue: false,
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
