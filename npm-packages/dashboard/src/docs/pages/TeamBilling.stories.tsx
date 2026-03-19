import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked } from "storybook/test";
import { useGetCurrentSpend } from "api/billing";
import { BillingPage } from "../../pages/t/[team]/settings/billing";

const meta = {
  component: BillingPage,
  parameters: {
    layout: "fullscreen",
  },
  beforeEach: () => {
    mocked(useGetCurrentSpend).mockReturnValue({
      totalCents: 1250,
      isLoading: false,
    });
  },
} satisfies Meta<typeof BillingPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
