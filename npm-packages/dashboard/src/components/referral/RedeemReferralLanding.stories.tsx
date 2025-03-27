import type { Meta, StoryObj } from "@storybook/react";
import { RedeemReferralLanding } from "./RedeemReferralLanding";

const meta = {
  component: RedeemReferralLanding,
  args: {
    title: "Someone thinks you’re a great fit for Convex!",
    code: "CONVEX123",
  },
} satisfies Meta<typeof RedeemReferralLanding>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
