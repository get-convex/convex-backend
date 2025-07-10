import { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { FilterEditor } from "@common/features/data/components/FilterEditor/FilterEditor";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0")
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

export default {
  component: FilterEditor,
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <FilterEditor {...args} />
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} as Meta<typeof FilterEditor>;

export const Editor: StoryObj<typeof FilterEditor> = {
  args: {
    fields: ["_id", "_creationTime", "myColumn"],
    defaultDocument: { myColumn: 0 },
  },
};
