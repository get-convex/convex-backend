import { Meta, StoryObj } from "@storybook/react";
import { ConvexProvider } from "convex/react";
import { ComponentProps } from "react";
import udfs from "udfs";
import { DataFilters } from "@common/features/data/components/DataFilters/DataFilters";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.listById.default, ({ ids }) => ids.map(() => null))
  .registerQueryFake(udfs.getVersion.default, () => "0.19.0");

export default {
  component: DataFilters,
  render: (args) => <Example {...args} />,
} as Meta<typeof DataFilters>;

function Example(args: ComponentProps<typeof DataFilters>) {
  return (
    <ConvexProvider client={mockClient}>
      <DataFilters
        {...args}
        filters={{ clauses: [] }}
        // eslint-disable-next-line no-alert
        onChangeFilters={() => alert("Filters applied!")}
      />
    </ConvexProvider>
  );
}

export const Default: StoryObj<typeof DataFilters> = {
  args: {
    tableName: "myTable",
    defaultDocument: { myColumn: 0 },
  },
};
