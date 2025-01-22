import { query } from "./_generated/server";

export const myConstantString = query({
  handler: () => {
    return "My never changing string";
  },
});
