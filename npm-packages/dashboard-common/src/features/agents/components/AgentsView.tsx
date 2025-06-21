import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { LoadingTransition } from "@ui/Loading";
import { PageContent } from "@common/elements/PageContent";
import { useNents } from "@common/lib/useNents";
import { Agents } from "@common/features/agents/components/Agents";

export function AgentsView() {
  const { nents } = useNents();
  return (
    <PageContent>
      <DeploymentPageTitle title="AI Agents" />
      <LoadingTransition>{nents && <Agents nents={nents} />}</LoadingTransition>
    </PageContent>
  );
}
