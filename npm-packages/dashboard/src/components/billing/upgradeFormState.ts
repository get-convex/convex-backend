import { CreateSubscriptionArgs } from "generatedApi";
import { SpendingLimitsValue } from "./SpendingLimits";

export type UpgradeFormState = CreateSubscriptionArgs &
  SpendingLimitsValue & { promoCode?: string };
