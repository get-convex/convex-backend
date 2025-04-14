import { LoadingTransition } from "@ui/Loading";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { useNents } from "@common/lib/useNents";
import { Components } from "@common/features/settings/components/Components";

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
