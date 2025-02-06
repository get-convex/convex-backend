import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { Callout } from "dashboard-common/elements/Callout";
import { UpgradePlanContent } from "./UpgradePlanContent";

export default {
  component: UpgradePlanContent,
  render: (args) => (
    <>
      <Callout className="mb-4 w-48">
        Inputs do not work in this storybook preview. Change the formState prop
        to see different states.
      </Callout>
      <UpgradePlanContent
        {...args}
        plan={{
          name: "Professional",
          id: "CONVEX_PROFESSIONAL",
          description: "The professional plan.",
          status: "active",
          seatPrice: 25,
          planType: "CONVEX_PROFESSIONAL",
        }}
        numMembers={2}
        paymentDetailsForm={
          <Callout className="w-fit">
            STRIPE PAYMENT DETAILS FORM WOULD BE HERE!
          </Callout>
        }
      />
    </>
  ),
} as Meta<typeof UpgradePlanContent>;

export const NoPaymentMethod: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: undefined,
        planId: "",
        email: "",
        name: "",
      },
      errors: {},
    },
  },
};

export const HasPaymentMethod: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: "abc",
        planId: "",
        email: "",
        name: "",
      },
      errors: {},
    },
  },
};

export const WithDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: "abc",
        planId: "",
        email: "",
        name: "",
      },
      errors: {},
    },
    teamMemberDiscountPct: 0.5,
    defaultPromoCode: "50OFF",
  },
};

export const WithPhasedDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: "abc",
        planId: "",
        email: "",
        name: "",
      },
      errors: {},
    },
    teamMemberDiscountPct: 0.5,
    couponDurationInMonths: 2,
    defaultPromoCode: "50OFF",
  },
};

export const WithUsageExemptDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: "abc",
        planId: "",
        email: "",
        name: "",
      },
      errors: {},
    },
    teamMemberDiscountPct: 0.5,
    couponDurationInMonths: 2,
    isUsageBasedBillingExempt: true,
    defaultPromoCode: "50OFF",
  },
};

export const WithFreeDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: "abc",
        planId: "",
        email: "",
        name: "",
      },
      errors: {},
    },
    teamMemberDiscountPct: 1,
    defaultPromoCode: "TOTALLY_FREE",
  },
};

export const WithFreeDiscountAndNoPaymentMethod: StoryObj<
  typeof UpgradePlanContent
> = {
  args: {
    teamMemberDiscountPct: 1,
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: undefined,
        planId: "",
        email: "",
        name: "",
        promoCode: "TOTALLY_FREE",
      },
      errors: {},
    },
  },
};

export const LoadingPromo: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: undefined,
        planId: "",
        email: "",
        name: "",
        promoCode: "LOADING",
      },
      errors: {},
    },
    isLoadingPromo: true,
  },
};

export const InvalidPromo: StoryObj<typeof UpgradePlanContent> = {
  args: {
    // @ts-expect-error
    formState: {
      values: {
        paymentMethod: undefined,
        planId: "",
        email: "",
        name: "",
        promoCode: "INVALID",
      },
      errors: {},
    },
    promoCodeError: "Invalid promo code.",
  },
};
