import React from "react";
import { Meta, StoryObj } from "@storybook/react";
import { Callout } from "dashboard-common/elements/Callout";
import { Formik } from "formik";
import { Sheet } from "dashboard-common/elements/Sheet";
import {
  UpgradePlanContent,
  UpgradePlanContentProps,
} from "./UpgradePlanContent";
import { UpgradeFormState } from "./upgradeFormState";

const DEFAULT_FORM_STATE: UpgradeFormState = {
  email: "",
  name: "",
  planId: "",
  paymentMethod: "abc",
  spendingLimitEnabled: false,
  spendingLimitDisableThresholdUsd: null,
  spendingLimitWarningThresholdUsd: null,
};

export default {
  component: UpgradePlanContent,
  render: (args) => render(args, DEFAULT_FORM_STATE),
} as Meta<typeof UpgradePlanContent>;

function render(args: UpgradePlanContentProps, formState: UpgradeFormState) {
  return (
    <Sheet>
      <Callout className="mb-4">
        Inputs do not work in this storybook preview. Change the formState prop
        to see different states.
      </Callout>
      <Formik initialValues={formState} onSubmit={() => {}}>
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
      </Formik>
    </Sheet>
  );
}

export const NoPaymentMethod: StoryObj<typeof UpgradePlanContent> = {
  args: {},
  render: (args) =>
    render(args, {
      ...DEFAULT_FORM_STATE,
      paymentMethod: undefined,
    }),
};

export const HasPaymentMethod: StoryObj<typeof UpgradePlanContent> = {
  args: {},
};

export const WithDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    teamMemberDiscountPct: 0.5,
  },
};

export const WithPhasedDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    teamMemberDiscountPct: 0.5,
    couponDurationInMonths: 2,
  },
};

export const WithFreeDiscount: StoryObj<typeof UpgradePlanContent> = {
  args: {
    teamMemberDiscountPct: 1,
  },
};

export const WithFreeDiscountAndNoPaymentMethod: StoryObj<
  typeof UpgradePlanContent
> = {
  render: (args) =>
    render(args, {
      ...DEFAULT_FORM_STATE,
      paymentMethod: undefined,
      promoCode: "TOTALLY_FREE",
    }),
  args: {
    teamMemberDiscountPct: 1,
  },
};

export const LoadingPromo: StoryObj<typeof UpgradePlanContent> = {
  render: (args) =>
    render(args, {
      ...DEFAULT_FORM_STATE,
      paymentMethod: undefined,
      promoCode: "LOADING",
    }),
  args: {
    isLoadingPromo: true,
  },
};

export const InvalidPromo: StoryObj<typeof UpgradePlanContent> = {
  render: (args) =>
    render(args, {
      ...DEFAULT_FORM_STATE,
      paymentMethod: undefined,
      promoCode: "INVALID",
    }),
  args: {
    promoCodeError: "Invalid promo code.",
  },
};
