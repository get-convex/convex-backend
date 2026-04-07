import { query, mutation, action } from "./_generated/server";

// exported helper function should be ignored since neither a mutation or query
export function ignoreMePls() {
  throw new Error("Why'd you call me??");
}

export const g = mutation(function g() {
  return 2;
});

// a mutation or query function can be exported multiple times
export const f1 = mutation(() => 3);
const f2 = f1;
export { f2 };

// This function should not be ignored even though it is not
// exported with its canonical name because it is explicitly
// marked as a query.
export const h = query(function not_h() {
  return 3;
});
// even default exports can appear multiple times
export default h;

export const action_in_v8 = action(() => 3);

// Regression test: if microtask queue isn't drained, this will panic.
void Promise.resolve(null).then(() => Date.now());
