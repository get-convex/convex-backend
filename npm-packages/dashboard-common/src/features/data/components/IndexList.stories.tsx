import { Meta, StoryObj } from "@storybook/nextjs";
import { IndexesList as IndexList } from "@common/features/data/components/IndexList";

const meta = {
  component: IndexList,
  args: {
    tableName: "messages",
  },
  render: (args) => (
    <div className="h-screen overflow-y-auto bg-background-secondary p-4">
      <IndexList {...args} />
    </div>
  ),
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof IndexList>;

export default meta;
type Story = StoryObj<typeof meta>;

export const LoadingStory: Story = {
  args: { indexes: undefined },
};

export const EmptyState: Story = {
  args: {
    indexes: [],
  },
};

export const WithIndexes: Story = {
  args: {
    indexes: [
      {
        table: "my-table",
        name: "by_author",
        fields: ["author"],
        backfill: { state: "done" },
      },
      {
        table: "my-table",
        name: "by_channel_and_message",
        fields: ["channel", "message"],
        backfill: { state: "done" },
      },
      {
        table: "my-table",
        name: "search_index",
        fields: {
          searchField: "body",
          filterFields: ["channel", "author"],
        },
        backfill: { state: "done" },
      },
      {
        table: "my-table",
        name: "vector_index",
        fields: {
          vectorField: "body",
          filterFields: ["channel", "author"],
          dimensions: 1536,
        },
        backfill: { state: "done" },
      },
    ],
  },
};

export const WithUpdatingIndexes: Story = {
  args: {
    indexes: [
      {
        table: "my-table",
        name: "new_index_no_stats",
        fields: ["author"],
        backfill: { state: "in_progress" },
      },
      {
        table: "my-table",
        name: "new_index_missing_total",
        fields: ["author"],
        backfill: {
          state: "in_progress",
          stats: { numDocsIndexed: 100, totalDocs: null },
        },
      },
      {
        table: "my-table",
        name: "new_index_with_stats",
        fields: ["author"],
        backfill: {
          state: "in_progress",
          stats: { numDocsIndexed: 100, totalDocs: 1000 },
        },
      },
      {
        table: "my-table",
        name: "updated_index",
        fields: ["name"],
        backfill: { state: "done" },
      },
      {
        table: "my-table",
        name: "updated_index",
        fields: ["name", "subtitle"],
        backfill: {
          state: "in_progress",
          stats: { numDocsIndexed: 500, totalDocs: 1000 },
        },
      },
      {
        table: "my-table",
        name: "updated_search_index",
        fields: { searchField: "title", filterFields: [] },
        backfill: { state: "done" },
      },
      {
        table: "my-table",
        name: "updated_search_index",
        fields: { searchField: "title", filterFields: ["author"] },
        backfill: {
          state: "in_progress",
          stats: { numDocsIndexed: 500, totalDocs: 1000 },
        },
      },
    ],
  },
};

export const WithLongIndex: Story = {
  args: {
    indexes: [
      {
        table: "my-table",
        name: "by_channel_and_message_and_author_and_modification_date",
        fields: ["channel", "message", "author", "modificationDate"],
        backfill: {
          state: "in_progress",
          stats: { numDocsIndexed: 0, totalDocs: 100 },
        },
      },
    ],
  },
};
