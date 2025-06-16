import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useBBMutation, useBBQuery } from "./api";

export function useTeamOrbSubscription(teamId?: number) {
  const {
    data: subscription,
    error,
    isLoading,
  } = useBBQuery({
    path: "/teams/{team_id}/get_orb_subscription",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    swrOptions: {
      refreshInterval: 0,
      keepPreviousData: false,
    },
  });
  if (error) {
    return { isLoading, subscription: null };
  }

  return { isLoading, subscription };
}

export function useCreateSetupIntent(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/create_setup_intent",
    pathParams: { team_id: teamId.toString() },
  });
}

export function useCreateSubscription(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/create_subscription",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast:
      "Congratulations! Your Convex subscription has been activated.",
  });
}

export function useChangeSubscription(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/change_subscription_plan",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Subscription changed.",
  });
}

export function useCancelSubscription(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/cancel_orb_subscription",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Subscription canceled.",
  });
}

export function useResumeSubscription(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/unschedule_cancel_orb_subscription",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Subscription resumed.",
  });
}

export function useUpdatePaymentMethod(teamId: number) {
  return useBBMutation({
    method: "put",
    path: "/teams/{team_id}/update_payment_method",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Payment method updated.",
  });
}

export function useUpdateBillingContact(teamId: number) {
  return useBBMutation({
    method: "put",
    path: "/teams/{team_id}/update_billing_contact",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Billing contact updated.",
  });
}

export function useUpdateBillingAddress(teamId: number) {
  return useBBMutation({
    method: "put",
    path: "/teams/{team_id}/update_billing_address",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_orb_subscription",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Billing address updated.",
  });
}

export function useGetCoupon(
  teamId: number,
  planId: string,
  promoCode?: string,
) {
  const { data, error, isLoading } = useBBQuery({
    path: "/teams/{team_id}/get_discounted_plan/{plan_id}/{promo_code}",
    pathParams: {
      team_id: teamId.toString(),
      plan_id: planId,
      promo_code: promoCode || "",
    },
    swrOptions: {
      refreshInterval: 0,
      shouldRetryOnError: false,
    },
  });

  if (error) {
    return {
      isLoading: false,
      errorMessage: "Failed to load coupon. Please try again.",
    };
  }

  return { coupon: data, isLoading: !!promoCode && isLoading };
}

export function useListInvoices(teamId?: number) {
  const { data, error, isLoading } = useBBQuery({
    path: "/teams/{team_id}/list_invoices",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    swrOptions: {
      refreshInterval: 0,
    },
  });

  if (error) {
    return {
      isLoading,
      invoices: [],
    };
  }

  return {
    invoices: data?.invoices.filter(
      (invoice) =>
        // Don't show test invoices from before we launched Orb billing.
        new Date(invoice.invoiceDate) >= new Date("2024-04-29"),
    ),
    isLoading,
  };
}

export function useListPlans(teamId?: number) {
  const { seatlessPlans } = useLaunchDarkly();

  const { data, error, isLoading } = useBBQuery({
    path: "/teams/{team_id}/list_active_plans",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    swrOptions: {
      refreshInterval: 0,
    },
  });

  if (error) {
    // eslint-disable-next-line @typescript-eslint/no-throw-literal
    throw error;
  }

  let plans = data?.plans;
  if (plans && !seatlessPlans) {
    plans = plans.filter((plan) => plan.seatPrice);
  }

  return {
    plans,
    isLoading,
  };
}

export function useGetCurrentSpend(teamId: number | null) {
  const { data, isLoading } = useBBQuery({
    path: "/teams/{team_id}/get_current_spend",
    pathParams: { team_id: teamId?.toString() ?? "" },
    swrOptions: {
      refreshInterval: 1000 * 60,
    },
  });

  return {
    totalCents: data?.totalCents,
    isLoading,
  };
}

export function useGetSpendingLimits(teamId: number | null): {
  spendingLimits:
    | {
        disableThresholdCents: number | null;
        state: null | "Running" | "Disabled" | "Warning";
        warningThresholdCents: number | null;
      }
    | undefined;
  isLoading: boolean;
} {
  const { data, isLoading } = useBBQuery({
    path: "/teams/{team_id}/get_spending_limits",
    pathParams: { team_id: teamId?.toString() ?? "" },
    swrOptions: {
      refreshInterval: 1000 * 60,
    },
  });

  return {
    spendingLimits:
      data === undefined
        ? undefined
        : {
            // The `?? null` checks are only necessary to fix an issue in the
            // OpenAPI codegen, the server always sends a value.
            state: data.state ?? null,
            disableThresholdCents: data.disableThresholdCents ?? null,
            warningThresholdCents: data.warningThresholdCents ?? null,
          },
    isLoading,
  };
}

export function useSetSpendingLimit(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/set_spending_limit",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/get_spending_limits",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Spending limit updated.",
  });
}
