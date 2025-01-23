import React from "react";
import { DirectorySidebar } from "components/functions/DirectorySidebar";
import { PerformanceGraphs } from "components/functions/PerformanceGraphs";
import {
  useCurrentOpenFunction,
  useModuleFunctions,
  EmptySection,
  SidebarDetailLayout,
} from "dashboard-common";
import { FileEditor } from "components/functions/FileEditor";
import { FunctionSummary } from "components/functions/FunctionSummary";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { CodeIcon } from "@radix-ui/react-icons";
import { useCurrentDeployment } from "api/deployments";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function FunctionsPage() {
  return (
    <>
      <DeploymentPageTitle title="Functions" />
      <FunctionsView />
    </>
  );
});

function FunctionsView() {
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
