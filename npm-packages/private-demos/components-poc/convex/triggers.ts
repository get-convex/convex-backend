import { internalMutation, mutation } from "./_generated/server";
import { components, internal } from "./_generated/api";
import { customMutation } from "convex-helpers/server/customFunctions";
import { WithTriggers, withTriggers } from "@convex-dev/triggers";
import { DataModel } from "./_generated/dataModel";

const mutationWrapper: WithTriggers<DataModel> = withTriggers(
  components.triggers,
  {
    notes: {
      atomicMutators: internal.notes,
      triggers: [internal.notes.onNote],
    },
  },
);

// Custom mutation that must be used to trigger the triggers.
export const mutationWithTriggers = customMutation(mutation, mutationWrapper);
export const internalMutationWithTriggers = customMutation(
  internalMutation,
  mutationWrapper,
);
