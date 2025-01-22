import { action } from "./_generated/server";
import { helper } from "./notSourceMappable";

export const nop = action(async () => {
  helper(1, 2);
});
