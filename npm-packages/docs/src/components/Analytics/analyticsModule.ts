import ExecutionEnvironment from "@docusaurus/ExecutionEnvironment";
import posthog from "posthog-js";

declare global {
  interface Window {
    posthog?: {
      capture(event: string): void;
    };
  }
}

export default (function () {
  if (!ExecutionEnvironment.canUseDOM) {
    return null;
  }

  return {
    onRouteUpdate({ location, previousLocation }) {
      if (location.pathname !== previousLocation?.pathname) {
        posthog.capture("$pageview");
      }
    },
  };
})();
