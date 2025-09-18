import { Meta, StoryObj } from "@storybook/nextjs";
import { fn } from "storybook/test";
import { SearchIndexFilterEditor } from "./SearchIndexFilterEditor";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/lib/values";
import { ConvexProvider } from "convex/react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import udfs from "@common/udfs";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

const meta: Meta<typeof SearchIndexFilterEditor> = {
  component: SearchIndexFilterEditor,
  args: {
    idx: 0,
    field: "foo",
    error: undefined,
    onChange: fn(),
    onApplyFilters: fn(),
    onError: fn(),
    filter: {
      field: "foo",
      enabled: false,
      value: UNDEFINED_PLACEHOLDER,
    },
    autoFocusValueEditor: false,
    documentValidator: undefined,
    shouldSurfaceValidatorErrors: false,
  },
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <SearchIndexFilterEditor {...args} />
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
};
export default meta;

type Story = StoryObj<typeof SearchIndexFilterEditor>;

export const Disabled: Story = {
  args: {
    filter: {
      field: "foo",
      enabled: false,
      value: UNDEFINED_PLACEHOLDER,
    },
  },
};

export const EnabledWithoutValue: Story = {
  args: {
    filter: {
      field: "foo",
      enabled: true,
      value: UNDEFINED_PLACEHOLDER,
    },
  },
};

export const EnabledWithValue: Story = {
  args: {
    filter: {
      field: "foo",
      enabled: true,
      value: { bar: "baz" },
    },
  },
};

export const WithError: Story = {
  args: {
    filter: {
      field: "foo",
      enabled: true,
      value: "some value",
    },
    error: "This is an error message.",
  },
};

export const CreationTimeField: Story = {
  args: {
    field: "_creationTime",
    filter: {
      field: "_creationTime",
      enabled: true,
      value: Date.now(),
    },
  },
};
