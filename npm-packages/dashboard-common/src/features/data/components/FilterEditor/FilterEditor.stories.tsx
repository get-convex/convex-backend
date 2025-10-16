import { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { FilterEditor } from "@common/features/data/components/FilterEditor/FilterEditor";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { fn } from "storybook/test";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0")
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

const meta = {
  component: FilterEditor,
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <FilterEditor {...args} />
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} satisfies Meta<typeof FilterEditor>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Editor: Story = {
  args: {
    fields: ["_id", "_creationTime", "myColumn"],
    defaultDocument: { myColumn: 0 },
    onChange: fn(),
    onDelete: fn(),
    onApplyFilters: fn(),
    onError: fn(),
  },
};
