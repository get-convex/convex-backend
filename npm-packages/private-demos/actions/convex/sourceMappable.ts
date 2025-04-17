import { action } from "./_generated/server";
import { helper } from "./notSourceMappable";

export const nop = action({
  args: {},
  handler: async () => {
    helper(1, 2);
  },
});
