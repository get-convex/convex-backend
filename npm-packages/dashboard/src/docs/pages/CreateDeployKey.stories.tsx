import { Meta, StoryObj } from "@storybook/nextjs";
import { userEvent, within } from "storybook/test";
import { CreateDeployKeyForm } from "../../components/deploymentSettings/GenerateDeployKeyButton";

const meta = {
  component: CreateDeployKeyForm,
  parameters: {
    layout: "fullscreen",
    // The deploy key permission picker is gated behind the `scopedDeployKeys`
    // LaunchDarkly flag. Force it on so the screenshot shows the permissions.
    docsPage: {
      launchDarkly: { scopedDeployKeys: true },
    },
    // Crop to the slide-in panel, which renders in a portal at document.body.
    screenshotSelector: '[data-testid="create-deploy-key-panel"]',
    a11y: { test: "todo" },
  },
  args: {
    deploymentType: "prod",
    disabledReason: null,
    showCustomPermissions: true,
    onClose: () => {},
    getAdminKey: async () => ({
      ok: true,
      adminKey: "prod:exciting-otter-123|eyJ2MiI6...",
    }),
  },
  render: (args) => <CreateDeployKeyForm {...args} />,
} satisfies Meta<typeof CreateDeployKeyForm>;

export default meta;
type Story = StoryObj<typeof meta>;

/**
 * The Create Deploy Key panel with the `deployment:deploy` permission selected,
 * as you would for a CI deploy key.
 */
export const Default: Story = {
  play: async () => {
    // The Headless UI Dialog renders in a portal at document.body, outside the
    // story's canvasElement.
    const body = within(document.body);
    const nameInput = await body.findByPlaceholderText(
      "Enter a memorable name for your deploy key",
    );
    await userEvent.type(nameInput, "CI Deploy Key");
    await userEvent.click(await body.findByText("deployment:deploy"));
  },
};
