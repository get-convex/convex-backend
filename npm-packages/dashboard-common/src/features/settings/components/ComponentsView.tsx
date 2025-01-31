import { LoadingTransition } from "elements/Loading";
import { DeploymentSettingsLayout } from "layouts/DeploymentSettingsLayout";
import { useNents } from "lib/useNents";
import { Components } from "features/settings/components/Components";

export function ComponentsView() {
  const { nents } = useNents();
  return (
    <DeploymentSettingsLayout page="components">
      <LoadingTransition>
        {nents && <Components nents={nents} />}
      </LoadingTransition>
    </DeploymentSettingsLayout>
  );
}
