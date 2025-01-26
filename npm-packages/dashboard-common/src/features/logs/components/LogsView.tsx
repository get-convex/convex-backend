import { LoadingTransition } from "../../../elements/Loading";
import { PageContent } from "../../../elements/PageContent";
import { useNents } from "../../../lib/useNents";
import { Logs } from "./Logs";

export function LogsView() {
  const { nents, selectedNent } = useNents();
  return (
    <PageContent>
      <LoadingTransition>
        {nents && <Logs nents={nents} selectedNent={selectedNent} />}
      </LoadingTransition>
    </PageContent>
  );
}
