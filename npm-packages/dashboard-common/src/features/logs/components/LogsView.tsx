import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { LoadingTransition } from "@ui/Loading";
import { PageContent } from "@common/elements/PageContent";
import { useNents } from "@common/lib/useNents";
import { Logs } from "@common/features/logs/components/Logs";

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
