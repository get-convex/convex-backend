import { query } from "../_generated/server";

// This is a test function that should be blocked for SDK versions < 1.28.2
export default query({
  args: {},
  handler: async () => {
    return { message: "betterAuth function executed successfully" };
  },
});
