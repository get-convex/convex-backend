import { action } from "./_generated/server";

export const doSomething = action({
  args: {},
  handler: () => {
    // implementation goes here

    // optionally return a value
    return "success";
  },
});
