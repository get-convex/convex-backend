import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { LoadingTransition } from "@ui/Loading";
import { PageContent } from "@common/elements/PageContent";
import { useNents } from "@common/lib/useNents";
import { Logs } from "@common/features/logs/components/Logs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useContext } from "react";

export function LogsView() {
  const { nents, selectedNent } = useNents();
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  return (
    <PageContent key={deployment?.id}>
      <DeploymentPageTitle title="Logs" />
      <LoadingTransition>
        {nents && <Logs nents={nents} selectedNent={selectedNent} />}
      </LoadingTransition>
    </PageContent>
  );
}
