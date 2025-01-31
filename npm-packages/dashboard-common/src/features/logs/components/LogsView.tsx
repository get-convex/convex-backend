import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { LoadingTransition } from "elements/Loading";
import { PageContent } from "elements/PageContent";
import { useNents } from "lib/useNents";
import { Logs } from "features/logs/components/Logs";

export function LogsView() {
  const { nents, selectedNent } = useNents();
  return (
    <PageContent>
      <DeploymentPageTitle title="Logs" />
      <LoadingTransition>
        {nents && <Logs nents={nents} selectedNent={selectedNent} />}
      </LoadingTransition>
    </PageContent>
  );
}
