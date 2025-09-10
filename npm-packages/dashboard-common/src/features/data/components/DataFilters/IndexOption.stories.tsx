import type { Meta, StoryObj } from "@storybook/nextjs";
import { IndexOption } from "./IndexFilters";

const meta: Meta<typeof IndexOption> = {
  component: IndexOption,
  parameters: {
    layout: "centered",
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const DefaultIndexByCreationTime: Story = {
  args: {
    label: "By creation time",
    value: {
      name: "by_creation_time",
      fields: ["_creationTime"],
      type: "default",
    },
    inButton: false,
  },
};

export const DefaultIndexById: Story = {
  args: {
    label: "By ID",
    value: {
      name: "by_id",
      fields: ["_id"],
      type: "default",
    },
    inButton: false,
  },
};

export const DefaultIndexButton: Story = {
  args: {
    label: "By ID",
    value: {
      name: "by_id",
      fields: ["_id"],
      type: "default",
    },
    inButton: true,
  },
};

export const DatabaseIndex: Story = {
  args: {
    label: "by_name_and_status",
    value: {
      name: "by_name_and_status",
      fields: ["name", "status"],
      type: "database",
    },
    inButton: false,
  },
};

export const DatabaseIndexButton: Story = {
  args: {
    label: "by_name_and_status",
    value: {
      name: "by_name_and_status",
      fields: ["name", "status"],
      type: "database",
    },
    inButton: true,
  },
};

export const SearchIndex: Story = {
  args: {
    label: "search_by_name",
    value: {
      name: "search_by_name",
      searchField: "name",
      fields: [],
      type: "search",
    },
    inButton: false,
  },
};

export const SearchIndexWithFilters: Story = {
  args: {
    label: "search_by_name_filtered_by_status",
    value: {
      name: "search_by_name_filtered_by_status",
      searchField: "name",
      fields: ["status"],
      type: "search",
    },
    inButton: false,
  },
};
