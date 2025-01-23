import { mutation, query } from "./_generated/server";

export const get = query(
  async ({ db }, { counterName }: { counterName: string }) => {
    const counterDoc = await db
      .query("counter_table")
      .filter((q) => q.eq(q.field("name"), counterName))
      .first();
    console.log(`Read counter ${counterName}:`, counterDoc);
    if (counterDoc === null) {
      return 0;
    }
    return counterDoc.counter;
  },
);

export const increment = mutation(
  async ({ db }, { counterName }: { counterName: string }) => {
    const counterDoc = await db
      .query("counter_table")
      .filter((q) => q.eq(q.field("name"), counterName))
      .first();
    const incrementBy = 1;
    if (counterDoc === null) {
      await db.insert("counter_table", {
        name: counterName,
        counter: incrementBy,
      });
      // console.log messages appear in your browser's console and the Convex dashboard.
      console.log("Created counter.");
    } else {
      counterDoc.counter += incrementBy;
      await db.replace(counterDoc._id, counterDoc);
      console.log(
        `Value of counter ${counterName} is now ${counterDoc.counter}.`,
      );
    }
  },
);
