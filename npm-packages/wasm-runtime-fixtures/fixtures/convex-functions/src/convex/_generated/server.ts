// Stands in for the file `npx convex codegen` writes into `convex/_generated`.
// The real generated file re-exports the same builders, typed against the app's
// data model; `convex-test` also uses the presence of a `_generated` path to
// locate the modules root.
export {
  actionGeneric as action,
  httpActionGeneric as httpAction,
  internalActionGeneric as internalAction,
  internalMutationGeneric as internalMutation,
  internalQueryGeneric as internalQuery,
  mutationGeneric as mutation,
  queryGeneric as query,
} from "convex/server";
