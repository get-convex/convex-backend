import { StoryObj } from "@storybook/react";
import { IndexesList as IndexList } from "features/data/components/IndexList";

export default { component: IndexList };

export const LoadingStory: StoryObj<typeof IndexList> = {
  args: { indexes: undefined },
};

export const EmptyState: StoryObj<typeof IndexList> = {
  args: {
    indexes: [],
  },
};

export const WithIndexes: StoryObj<typeof IndexList> = {
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
        backfill: { state: "in_progress" },
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
    ],
  },
};

export const WithLongIndex: StoryObj<typeof IndexList> = {
  args: {
    indexes: [
      {
        table: "my-table",
        name: "by_channel_and_message_and_author",
        fields: ["channel", "message", "author"],
        backfill: { state: "in_progress" },
      },
    ],
  },
};
