import { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import { ComponentProps, useMemo } from "react";
import udfs from "@common/udfs";
import { DataFilters } from "@common/features/data/components/DataFilters/DataFilters";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import {
  ConnectedDeploymentContext,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

// @ts-expect-error
const deployment: ConnectedDeployment = {};

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0")
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(
    udfs.indexes.default,
    ({ tableName: _tableName, tableNamespace: _tableNamespace }) => [],
  );

export default {
  component: DataFilters,
  render: (args) => <Example {...args} />,
} as Meta<typeof DataFilters>;

function Example(args: ComponentProps<typeof DataFilters>) {
  const connectedDeployment = useMemo(
    () => ({ deployment, isDisconnected: false }),
    [],
  );
  return (
    <ConnectedDeploymentContext.Provider value={connectedDeployment}>
      <ConvexProvider client={mockClient}>
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <DataFilters
            {...args}
            filters={{ clauses: [] }}
            // eslint-disable-next-line no-alert
            onChangeFilters={() => alert("Filters applied!")}
          />
        </DeploymentInfoContext.Provider>
      </ConvexProvider>
    </ConnectedDeploymentContext.Provider>
  );
}

export const Default: StoryObj<typeof DataFilters> = {
  args: {
    tableName: "myTable",
    defaultDocument: { myColumn: 0 },
  },
};
