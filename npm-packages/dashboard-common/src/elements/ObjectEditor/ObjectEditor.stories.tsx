import { Meta, StoryObj } from "@storybook/react";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { ObjectEditor } from "@common/elements/ObjectEditor/ObjectEditor";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => null);

export default {
  component: ObjectEditor,
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <div className="h-64">
        <ObjectEditor {...args} path="document" mode="editField" />
      </div>
    </ConvexProvider>
  ),
} as Meta<typeof ObjectEditor>;

export const Primary: StoryObj<typeof ObjectEditor> = {
  args: {
    defaultValue: null,
  },
};
