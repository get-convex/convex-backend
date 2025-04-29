import {
  FileIcon,
  TableIcon,
  CodeIcon,
  StopwatchIcon,
  CounterClockwiseClockIcon,
  TextAlignBottomIcon,
  GearIcon,
} from "@radix-ui/react-icons";
import { useQuery } from "convex/react";
import Link from "next/link";
import { useContext, useState } from "react";
import udfs from "@common/udfs";
import classNames from "classnames";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useCollapseSidebarState } from "@common/lib/useCollapseSidebarState";
import { PulseIcon } from "@common/elements/icons";
import { Sidebar } from "@common/elements/Sidebar";
import { FunctionRunnerWrapper } from "@common/features/functionRunner/components/FunctionRunnerWrapper";
import { FunctionsProvider } from "@common/lib/functions/FunctionsProvider";
import { useIsGlobalRunnerShown } from "@common/features/functionRunner/lib/functionRunner";
import { useIsCloudDeploymentInSelfHostedDashboard } from "@common/lib/useIsCloudDeploymentInSelfHostedDashboard";
import { BotIcon } from "@common/lib/logos/BotIcon";

type LayoutProps = {
  children: JSX.Element;
  auditLogsEnabled?: boolean;
};

export function DeploymentDashboardLayout({
  children,
  auditLogsEnabled = true,
}: LayoutProps) {
  const [collapsed, setCollapsed] = useCollapseSidebarState();
  const [isGlobalRunnerVertical, setIsGlobalRunnerVertical] =
    useGlobalLocalStorage("functionRunnerOrientation", false);
  const [isRunnerExpanded, setIsRunnerExpanded] = useState(false);
  const isGlobalRunnerShown = useIsGlobalRunnerShown();
  const { deploymentsURI: uriPrefix } = useContext(DeploymentInfoContext);
  const { isCloudDeploymentInSelfHostedDashboard, deploymentName } =
    useIsCloudDeploymentInSelfHostedDashboard();

  const exploreDeploymentPages = [
    {
      key: null,
      label: "Health",
      Icon: PulseIcon,
      href: `${uriPrefix}/`,
    },
    {
      key: "data",
      label: "Data",
      Icon: TableIcon,
      href: `${uriPrefix}/data`,
    },
    {
      key: `functions`,
      label: "Functions",
      Icon: CodeIcon,
      href: `${uriPrefix}/functions`,
    },
    {
      key: "files",
      label: "Files",
      Icon: FileIcon,
      href: `${uriPrefix}/files`,
    },
    {
      key: "schedules",
      label: "Schedules",
      Icon: StopwatchIcon,
      href: `${uriPrefix}/schedules/functions`,
    },
    {
      key: "logs",
      label: "Logs",
      Icon: (props: any) => (
        <TextAlignBottomIcon {...props} style={{ marginTop: "-4px" }} />
      ),
      href: `${uriPrefix}/logs`,
    },
    {
      key: "agent",
      label: "AI Agents",
      Icon: (props: any) => <BotIcon {...props} />,
      href: `${uriPrefix}/agents`,
    },
  ];

  const sidebarItems = [
    {
      key: "explore",
      items: exploreDeploymentPages,
    },
    {
      key: "configure",
      items: [
        {
          key: "history",
          label: "History",
          Icon: CounterClockwiseClockIcon,
          href: isCloudDeploymentInSelfHostedDashboard
            ? `https://dashboard.convex.dev/d/${deploymentName}/history`
            : `${uriPrefix}/history`,
          target: isCloudDeploymentInSelfHostedDashboard ? "_blank" : undefined,
          disabled: !auditLogsEnabled,
          tooltip: auditLogsEnabled
            ? undefined
            : "Deployment history is only available on paid plans.",
        },
        {
          key: "settings",
          label: "Settings",
          Icon: GearIcon,
          href: `${uriPrefix}/settings`,
        },
      ],
    },
  ];

  return (
    <FunctionsProvider>
      <div className="flex h-full grow flex-col overflow-y-hidden">
        <PauseBanner />
        <div className="flex h-full flex-col sm:flex-row">
          <Sidebar
            collapsed={!!collapsed}
            setCollapsed={setCollapsed}
            items={sidebarItems}
          />
          <div
            className={classNames(
              "flex w-full grow overflow-x-hidden",
              !isGlobalRunnerVertical && "flex-col"
            )}
          >
            {/* If the function runner is fully expanded, hide the content */}
            <div
              className={
                isRunnerExpanded && isGlobalRunnerShown
                  ? "h-0 w-0"
                  : "h-full w-full overflow-x-auto scrollbar"
              }
            >
              {children}
            </div>
            <FunctionRunnerWrapper
              setIsVertical={setIsGlobalRunnerVertical}
              isVertical={!!isGlobalRunnerVertical}
              isExpanded={isRunnerExpanded}
              setIsExpanded={setIsRunnerExpanded}
            />
          </div>
        </div>
      </div>
    </FunctionsProvider>
  );
}

function PauseBanner() {
  const deploymentState = useQuery(udfs.deploymentState.deploymentState);

  const { useCurrentTeam, useCurrentUsageBanner } = useContext(
    DeploymentInfoContext
  );

  const team = useCurrentTeam();
  const teamUsageBanner = useCurrentUsageBanner(team?.id ?? null);

  const { deploymentsURI } = useContext(DeploymentInfoContext);

  if (!(deploymentState?.state === "paused" && teamUsageBanner !== "Paused")) {
    return null;
  }

  return (
    <div className="bg-background-error py-2 text-center text-content-error">
      This deployment is paused. Resume your deployment on the{" "}
      <Link
        passHref
        href={`${deploymentsURI}/settings/pause-deployment`}
        className="text-content-link hover:underline"
      >
        settings
      </Link>{" "}
      page.
    </div>
  );
}
