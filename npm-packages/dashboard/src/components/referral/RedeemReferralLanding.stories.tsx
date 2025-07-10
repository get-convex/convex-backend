import type { Meta, StoryObj } from "@storybook/nextjs";
import { RedeemReferralLanding } from "./RedeemReferralLanding";

const meta = {
  component: RedeemReferralLanding,
  args: {
    title: "Someone thinks you’d like Convex!",
    code: "CONVEX123",
    isChef: false,
  },
} satisfies Meta<typeof RedeemReferralLanding>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
