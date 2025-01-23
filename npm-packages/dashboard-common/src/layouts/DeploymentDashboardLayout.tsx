import {
  FileIcon,
  TableIcon,
  MixerHorizontalIcon,
  CodeIcon,
  StopwatchIcon,
  CounterClockwiseClockIcon,
  TextAlignBottomIcon,
} from "@radix-ui/react-icons";
import { useRouter } from "next/router";
import { useQuery } from "convex/react";
import Link from "next/link";
import { useContext, useState } from "react";
import udfs from "udfs";
import classNames from "classnames";
import { DeploymentInfoContext } from "../lib/deploymentContext";
import { useGlobalLocalStorage } from "../lib/useGlobalLocalStorage";
import { useCollapseSidebarState } from "../lib/useCollapseSidebarState";
import { PulseIcon } from "../elements/icons";
import { Sidebar } from "../elements/Sidebar";
import { FunctionRunnerWrapper } from "../features/functionRunner/components/FunctionRunnerWrapper";
import { FunctionsProvider } from "../lib/functions/FunctionsProvider";

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
  const { deploymentsURI: uriPrefix } = useContext(DeploymentInfoContext);

  const exploreDeploymentPages = [
    {
      key: null,
      label: "Health",
      Icon: PulseIcon,
      href: uriPrefix,
    },
    {
      key: "data",
      label: "Data",
      Icon: TableIcon,
      href: `${uriPrefix}data`,
    },
    {
      key: `functions`,
      label: "Functions",
      Icon: CodeIcon,
      href: `${uriPrefix}functions`,
    },
    {
      key: "files",
      label: "Files",
      Icon: FileIcon,
      href: `${uriPrefix}files`,
    },
    {
      key: "schedules",
      label: "Schedules",
      Icon: StopwatchIcon,
      href: `${uriPrefix}schedules/functions`,
    },
    {
      key: "logs",
      label: "Logs",
      Icon: (props: any) => (
        <TextAlignBottomIcon {...props} style={{ marginTop: "-4px" }} />
      ),
      href: `${uriPrefix}logs`,
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
          href: `${uriPrefix}history`,
          disabled: !auditLogsEnabled,
          tooltip: auditLogsEnabled
            ? undefined
            : "Deployment history is only available on paid plans.",
        },
        {
          key: "settings",
          label: "Settings",
          Icon: MixerHorizontalIcon,
          href: `${uriPrefix}settings`,
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
              !isGlobalRunnerVertical && "flex-col",
            )}
          >
            {/* If the function runner is fully expanded, hide the content */}
            <div
              className={
                isRunnerExpanded
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
  const { query } = useRouter();
  const teamSlug = query.team as string;
  const projectSlug = query.project as string;
  const deploymentName = query.deploymentName as string;
  const deploymentState = useQuery(udfs.deploymentState.deploymentState);

  const { useCurrentTeam, useCurrentUsageBanner } = useContext(
    DeploymentInfoContext,
  );

  const team = useCurrentTeam();
  const teamUsageBanner = useCurrentUsageBanner(team?.id ?? null);

  if (!(deploymentState?.state === "paused" && teamUsageBanner !== "Paused")) {
    return null;
  }

  return (
    <div className="bg-background-error py-2 text-center text-content-error">
      This deployment is paused. Resume your deployment on the{" "}
      <Link
        passHref
        href={`/t/${teamSlug}/${projectSlug}/${deploymentName}/settings/pause-deployment`}
        className="text-content-link hover:underline dark:underline"
      >
        settings
      </Link>{" "}
      page.
    </div>
  );
}
