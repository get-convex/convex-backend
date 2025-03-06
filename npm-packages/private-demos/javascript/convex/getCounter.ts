import { query } from "./_generated/server";
import { foo } from "../src/utils.js";

export default query({
  handler: async ({ db }): Promise<number> => {
    const counterDoc = await db.query("counter_table").first();
    console.log("Got stuff");
    if (counterDoc === null) {
      return 0;
    }
    foo();
    return counterDoc.counter;
  },
});
