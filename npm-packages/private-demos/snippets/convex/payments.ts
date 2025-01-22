import { v } from "convex/values";
import { internalMutation } from "./_generated/server";

export const sendPaymentEmail = internalMutation({
  args: { email: v.string() },
  handler: async () => {
    // empty
  },
});
