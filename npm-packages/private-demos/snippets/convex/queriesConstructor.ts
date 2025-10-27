import { query } from "./_generated/server";

export const myConstantString = query({
  args: {},
  handler: () => {
    return "My never changing string";
  },
});
