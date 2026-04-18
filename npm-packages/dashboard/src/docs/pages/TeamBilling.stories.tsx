import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
import { useGetCurrentSpend, useCreateSetupIntent } from "api/billing";
import { BillingPage } from "../../pages/t/[team]/settings/billing";

const meta = {
  component: BillingPage,
  parameters: {
    layout: "fullscreen",
    a11y: {
      test: "todo",
    },
  },
  beforeEach: () => {
    mocked(useGetCurrentSpend).mockReturnValue({
      totalCents: 1250,
      isLoading: false,
    });
    mocked(useCreateSetupIntent).mockReturnValue(async () => ({
      clientSecret: "test_secret",
    }));
  },
} satisfies Meta<typeof BillingPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
