import { Meta, StoryObj } from "@storybook/react";
import { ConvexProvider } from "convex/react";
import udfs from "udfs";
import { FilterEditor } from "features/data/components/FilterEditor/FilterEditor";
import { mockConvexReactClient } from "lib/mockConvexReactClient";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0")
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

export default {
  component: FilterEditor,
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <FilterEditor {...args} />
    </ConvexProvider>
  ),
} as Meta<typeof FilterEditor>;

export const Editor: StoryObj<typeof FilterEditor> = {
  args: {
    fields: ["_id", "_creationTime", "myColumn"],
    defaultDocument: { myColumn: 0 },
  },
};
