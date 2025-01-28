import { useContext } from "react";
import { CodeIcon } from "@radix-ui/react-icons";
import {
  useCurrentOpenFunction,
  useModuleFunctions,
} from "../../../lib/functions/FunctionsProvider";
import { DirectorySidebar } from "./DirectorySidebar";
import { FileEditor } from "./FileEditor";
import { FunctionSummary } from "./FunctionSummary";
import { PerformanceGraphs } from "./PerformanceGraphs";
import { DeploymentInfoContext } from "../../../lib/deploymentContext";
import { SidebarDetailLayout } from "../../../layouts/SidebarDetailLayout";
import { EmptySection } from "../../../elements/EmptySection";
import { DeploymentPageTitle } from "../../../elements/DeploymentPageTitle";

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
      <div className="flex h-fit max-w-[110rem] flex-col gap-3 p-6 py-4">
        <div className="flex-none">
          <FunctionSummary currentOpenFunction={currentOpenFunction} />
        </div>
        <div className="flex-none">
          <PerformanceGraphs />
        </div>
        <div>
          <FileEditor moduleFunction={currentOpenFunction} />
        </div>
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
