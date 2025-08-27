import type { Meta, StoryObj } from "@storybook/nextjs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { ConvexProvider } from "convex/react";
import udfs from "@common/udfs";
import { IndexFilters } from "./IndexFilters";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

const meta: Meta<typeof IndexFilters> = {
  component: IndexFilters,

  args: {
    shownFilters: {
      clauses: [],
      index: {
        name: "by_creation_time",
        clauses: [
          {
            type: "indexRange",
            enabled: false,
            lowerOp: "gte",
            lowerValue: new Date().getTime(),
          },
        ],
      },
      order: "asc",
    },
    defaultDocument: {
      _id: "123",
      _creationTime: new Date().getTime(),
      name: "Sample Document",
      status: "active",
    },
    indexes: [
      {
        name: "by_status",
        fields: ["status"],
        staged: false,
        backfill: {
          state: "done",
        },
      },
      {
        name: "by_name_status",
        fields: ["name", "status"],
        staged: false,
        backfill: {
          state: "done",
        },
      },
    ],
    tableName: "users",
    activeSchema: {
      schemaValidation: true,
      tables: [
        {
          tableName: "users",
          indexes: [
            {
              indexDescriptor: "by_status",
              fields: ["status"],
            },
            {
              indexDescriptor: "by_name_status",
              fields: ["name", "status"],
            },
          ],
          searchIndexes: [],
          documentType: null,
        },
      ],
    },
    getValidatorForField: () => ({ type: "string" }),
    onFiltersChange: () => {},
    applyFiltersWithHistory: async () => {},
    setDraftFilters: () => {},
    onChangeOrder: () => {},
    onChangeIndexFilter: () => {},
    onError: () => {},
    hasInvalidFilters: false,
    invalidFilters: {},
  },

  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <IndexFilters {...args} />
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
};

export default meta;
type Story = StoryObj<typeof meta>;

export const SystemIndexByCreationTime: Story = {};

export const SystemIndexById: Story = {
  args: {
    shownFilters: {
      clauses: [],
      index: {
        name: "by_id",
        clauses: [
          {
            type: "indexEq",
            enabled: true,
            value: "j57d6s1z8s9t2v3w4x5y6z7a8b9c0d1e",
          },
        ],
      },
      order: "asc",
    },
    defaultDocument: {
      _id: "j57d6s1z8s9t2v3w4x5y6z7a8b9c0d1e",
      _creationTime: new Date().getTime(),
      name: "Sample Document",
      status: "active",
    },
  },
};

export const DatabaseIndex: Story = {
  args: {
    shownFilters: {
      clauses: [],
      index: {
        name: "by_name_status",
        clauses: [
          {
            type: "indexEq",
            enabled: true,
            value: "John Doe",
          },
          {
            type: "indexEq",
            enabled: true,
            value: "active",
          },
        ],
      },
      order: "desc",
    },
    defaultDocument: {
      _id: "123",
      _creationTime: new Date().getTime(),
      name: "John Doe",
      status: "active",
    },
  },
};

export const DatabaseIndexPartialFilter: Story = {
  args: {
    shownFilters: {
      clauses: [],
      index: {
        name: "by_name_status",
        clauses: [
          {
            type: "indexEq",
            enabled: true,
            value: "John Doe",
          },
          {
            type: "indexEq",
            enabled: false,
            value: "",
          },
        ],
      },
      order: "asc",
    },
    defaultDocument: {
      _id: "456",
      _creationTime: new Date().getTime(),
      name: "John Doe",
      status: "pending",
    },
  },
};

export const WithError: Story = {
  args: {
    hasInvalidFilters: true,
    invalidFilters: {
      "index/0": "Invalid value format",
    },
    shownFilters: {
      clauses: [],
      index: {
        name: "by_status",
        clauses: [
          {
            type: "indexEq",
            enabled: true,
            value: "",
          },
        ],
      },
      order: "asc",
    },
  },
};
