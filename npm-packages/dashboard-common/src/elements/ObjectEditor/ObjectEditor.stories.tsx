import { Meta, StoryObj } from "@storybook/nextjs";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

export default {
  component: ObjectEditor,
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <div className="h-64">
          <ObjectEditor {...args} />
        </div>
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
} as Meta<typeof ObjectEditor>;

export const Primary: StoryObj<typeof ObjectEditor> = {
  args: {
    defaultValue: null,
    onChange: () => {},
    onError: () => {},
    path: "document",
    mode: "editField",
  },
};
