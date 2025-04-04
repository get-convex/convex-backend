import { useContext, useState } from "react";
import { CodeIcon } from "@radix-ui/react-icons";
import {
  useCurrentOpenFunction,
  useModuleFunctions,
} from "@common/lib/functions/FunctionsProvider";
import { DirectorySidebar } from "@common/features/functions/components/DirectorySidebar";
import { FileEditor } from "@common/features/functions/components/FileEditor";
import { FunctionSummary } from "@common/features/functions/components/FunctionSummary";
import { PerformanceGraphs } from "@common/features/functions/components/PerformanceGraphs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { SidebarDetailLayout } from "@common/layouts/SidebarDetailLayout";
import { EmptySection } from "@common/elements/EmptySection";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { Tab } from "@common/elements/Tab";
import { Tab as HeadlessTab } from "@headlessui/react";
import { useNents } from "@common/lib/useNents";
import { FunctionLogs } from "./FunctionLogs";

export function FunctionsView() {
  return (
    <>
      <DeploymentPageTitle title="Functions" />
      <Functions />
    </>
  );
}
function Functions() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deploymentId = useCurrentDeployment()?.id;
  const currentOpenFunction = useCurrentOpenFunction();
  const modules = useModuleFunctions();
  const [selectedTabIndex, setSelectedTabIndex] = useState(0);
  const { selectedNent } = useNents();

  if (modules.length === 0) {
    return <EmptyFunctions />;
  }

  let content: React.ReactNode;
  if (!currentOpenFunction) {
    content = (
      <span className="grid h-full w-full place-content-center">
        Select a function on the left to open it.
      </span>
    );
  } else {
    content = (
      <div className="flex h-full max-w-[110rem] grow flex-col">
        <div className="flex-none bg-background-secondary px-6 pt-4">
          <FunctionSummary currentOpenFunction={currentOpenFunction} />
        </div>

        <HeadlessTab.Group
          selectedIndex={selectedTabIndex}
          onChange={setSelectedTabIndex}
          className="flex grow flex-col"
          as="div"
        >
          <div className="-ml-2 mb-6 flex gap-2 border-b bg-background-secondary px-6">
            <Tab>Statistics</Tab>
            <Tab>Code</Tab>
            <Tab>Logs</Tab>
          </div>

          <HeadlessTab.Panels className="flex w-full grow px-6 pb-4">
            <HeadlessTab.Panel className="grow">
              <PerformanceGraphs />
            </HeadlessTab.Panel>

            <HeadlessTab.Panel className="grow">
              <FileEditor moduleFunction={currentOpenFunction} />
            </HeadlessTab.Panel>

            <HeadlessTab.Panel className="grow">
              <FunctionLogs
                key={currentOpenFunction.displayName}
                currentOpenFunction={currentOpenFunction}
                selectedNent={selectedNent || undefined}
              />
            </HeadlessTab.Panel>
          </HeadlessTab.Panels>
        </HeadlessTab.Group>
      </div>
    );
  }

  return (
    <SidebarDetailLayout
      panelSizeKey={`${deploymentId}/functions`}
      sidebarComponent={<DirectorySidebar onChangeFunction={() => {}} />}
      contentComponent={content}
    />
  );
}

function EmptyFunctions() {
  return (
    <div className="flex h-full w-full items-center justify-center p-6">
      <EmptySection
        Icon={CodeIcon}
        color="yellow"
        header="No functions in this deployment, yet."
        body="Deploy some functions to get started."
        learnMoreButton={{
          href: "https://docs.convex.dev/quickstarts",
          children: "Follow a quickstart guide for your favorite framework.",
        }}
        sheet={false}
      />
    </div>
  );
}
