import { mutation, query } from "./_generated/server";

export const get = query(
  async ({ db }, { counterName }: { counterName: string }): Promise<number> => {
    const counterDoc = await db
      .query("counter_table")
      .filter((q) => q.eq(q.field("name"), counterName))
      .first();
    console.log("Got stuff");
    if (counterDoc === null) {
      return 0;
    }
    return counterDoc.counter;
  },
);

export const increment = mutation(
  async (
    { db, auth },
    { counterName, increment }: { counterName: string; increment: number },
  ) => {
    const identity = await auth.getUserIdentity();
    if (!identity) {
      throw new Error("Unauthenticated call to incrementCounter");
    }
    const counterDoc = await db
      .query("counter_table")
      .filter((q) => q.eq(q.field("name"), counterName))
      .first();
    if (counterDoc === null) {
      await db.insert("counter_table", {
        name: counterName,
        counter: increment,
      });
      // console.log messages appear in your browser's console and the Convex dashboard.
      console.log("Created counter.");
    } else {
      counterDoc.counter += increment;
      await db.replace(counterDoc._id, counterDoc);
      console.log(`Value of counter is now ${counterDoc.counter}.`);
    }
  },
);
