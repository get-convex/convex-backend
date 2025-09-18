import { Meta, StoryObj } from "@storybook/nextjs";
import { fn } from "storybook/test";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/lib/values";
import udfs from "@common/udfs";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { ConvexProvider } from "convex/react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { ObjectEditorWithPlaceholder } from "./ObjectEditorWithPlaceholder";

const mockClient = mockConvexReactClient()
  .registerQueryFake(udfs.components.list, () => [])
  .registerQueryFake(udfs.getTableMapping.default, () => ({}));

const meta: Meta<typeof ObjectEditorWithPlaceholder> = {
  component: ObjectEditorWithPlaceholder,
  args: {
    onChangeHandler: fn(),
    onApplyFilters: fn(),
    handleError: fn(),
    path: "test-path",
    className: "",
    documentValidator: undefined,
    shouldSurfaceValidatorErrors: false,
  },
  render: (args) => (
    <ConvexProvider client={mockClient}>
      <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
        <ObjectEditorWithPlaceholder {...args} />
      </DeploymentInfoContext.Provider>
    </ConvexProvider>
  ),
};
export default meta;

type Story = StoryObj<typeof ObjectEditorWithPlaceholder>;

export const Disabled: Story = {
  args: {
    enabled: false,
    value: UNDEFINED_PLACEHOLDER,
  },
};

export const EnabledWithoutValue: Story = {
  args: {
    enabled: true,
    value: UNDEFINED_PLACEHOLDER,
  },
};

export const EnabledWithValue: Story = {
  args: {
    enabled: true,
    value: { foo: "bar" },
  },
};
