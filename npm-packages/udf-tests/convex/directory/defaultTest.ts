import { query } from "../_generated/server";
import { fibonacci } from "../helpers.js";

export default query((_, { a }: { a: number }) => {
  return 2 * fibonacci(a);
});
