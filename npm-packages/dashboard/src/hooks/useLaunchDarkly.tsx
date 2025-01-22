import { useFlags } from "launchdarkly-react-client-sdk";
import kebabCase from "lodash/kebabCase";

const flagDefaults = {
  dataViewPageSize: 25,
  dataViewPaginationThreshold: 25,
  dataPageInactivityTime: -1,
  localDeployments: false,
  askAi: false,
};

function kebabCaseKeys(object: typeof flagDefaults) {
  return Object.entries(object).reduce(
    (carry, [key, value]) => ({ ...carry, [kebabCase(key)]: value }),
    {} as { [key: string]: any },
  );
}

// Flag defaults need to be in the default kebab-case format:
// https://docs.launchdarkly.com/sdk/client-side/react/react-web#configuring-the-react-sdk
export const flagDefaultsKebabCase = kebabCaseKeys(flagDefaults);

// useLaunchDarkly is a thin wrapper on LaunchDarkly's react sdk which adds manual to flag keys.
// At some point, we can generate this file.
export function useLaunchDarkly() {
  return useFlags<typeof flagDefaults>();
}
