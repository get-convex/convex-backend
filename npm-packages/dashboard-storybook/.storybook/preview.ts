import "./preview.css";
import { Preview } from "@storybook/nextjs";
import themeDecorator from "./themeDecorator";
import { docsPageDecorator } from "./docsPageDecorator";
import { sb } from "storybook/test";

// Register modules for mocking in stories
// Note: paths must be relative to this file and include extensions for Node.js resolution
sb.mock(import("../../dashboard/src/api/teams.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/projects.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/profile.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/deployments.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/roles.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/invitations.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/billing.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/accessTokens.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/backups.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/vanityDomains.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/environmentVariables.ts"), {
  spy: true,
});
sb.mock(import("../../dashboard/src/api/optins.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/api.ts"), { spy: true });
sb.mock(import("../../dashboard/src/hooks/useLaunchDarkly.tsx"), { spy: true });
sb.mock(import("../../dashboard/src/hooks/usePostHog.ts"), { spy: true });
sb.mock(import("../../dashboard/src/hooks/useServerSideData.ts"), {
  spy: true,
});
sb.mock(import("../../dashboard/src/api/usage.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/auditLog.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/oauth.ts"), { spy: true });
sb.mock(import("../../dashboard/src/lib/deploymentAuth.ts"), { spy: true });
sb.mock(import("../../dashboard/src/hooks/usageMetrics.ts"), { spy: true });
sb.mock(import("../../dashboard/src/hooks/usageMetricsV2.ts"), { spy: true });
sb.mock(import("../../dashboard-common/src/elements/LocalDevCallout.tsx"), {
  spy: true,
});
sb.mock(
  import(
    "../../dashboard-common/src/features/disconnectOverlay/CloudDisconnectOverlay.tsx"
  ),
  {
    spy: true,
  },
);
sb.mock(
  import("../../dashboard/src/components/projectSettings/CustomDomains.tsx"),
  { spy: true },
);
sb.mock(import("../../dashboard-common/src/lib/deploymentApi.ts"), {
  spy: true,
});

const preview: Preview = {
  initialGlobals: {
    a11y: { manual: false },
  },
  parameters: {
    a11y: { test: "error" },
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
  },

  decorators: [
    themeDecorator({
      themes: {
        light: "light",
        dark: "dark",
      },
      defaultTheme: "light",
    }),
    docsPageDecorator,
  ],
};

export default preview;
