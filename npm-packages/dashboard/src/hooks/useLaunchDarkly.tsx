import { useFlags } from "launchdarkly-react-client-sdk";
import kebabCase from "lodash/kebabCase";

export const flagDefaults: {
  commandPaletteDeleteProjects: boolean;
  enableNewDashboardVersionNotification: boolean;
  connectionStateCheckIntervalMs: number;
  nonDefaultTeamDevsInMainMenu: number;
  showAiGatewayUsage: boolean;
} = {
  commandPaletteDeleteProjects: false,
  enableNewDashboardVersionNotification: false,
  connectionStateCheckIntervalMs: 2500,
  nonDefaultTeamDevsInMainMenu: 10,
  showAiGatewayUsage: false,
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
