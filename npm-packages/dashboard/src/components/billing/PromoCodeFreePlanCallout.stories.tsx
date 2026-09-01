import { Meta, StoryObj } from "@storybook/nextjs";
import { PromoCodeFreePlanCallout } from "./SubscriptionOverview";

const meta = {
  component: PromoCodeFreePlanCallout,
} satisfies Meta<typeof PromoCodeFreePlanCallout>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
