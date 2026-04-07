import { Id } from "./_generated/dataModel";
import { mutation, query } from "./_generated/server";

export const initialize = mutation({
  handler: async (
    { db },
    { name, balance }: { name: string; balance: number },
  ) => {
    await db.insert("accounts", { name, balance });
  },
});

export const deposit = mutation({
  handler: async (
    { db },
    { name, balance }: { name: string; balance: number },
  ) => {
    const doc = await db
      .query("accounts")
      .filter((q) => q.eq(q.field("name"), name))
      .unique();
    if (doc === null) {
      throw new Error("Expected exactly one account with name");
    }
    doc.balance += balance;
    await db.replace(doc._id as Id<any>, doc);
    // return a result so we can test that functionality too
    return name + "'s balance is now " + doc.balance;
  },
});

export const accountBalance = query({
  handler: async ({ db }, { name }: { name: string }) => {
    const doc = await db
      .query("accounts")
      .filter((q) => q.eq(q.field("name"), name))
      .first();
    return doc?.balance ?? 0;
  },
});

export const transfer = mutation({
  handler: async (
    { db },
    { from, to, amount }: { from: string; to: string; amount: number },
  ) => {
    const fromDoc = (await db
      .query("accounts")
      .filter((q) => q.eq(q.field("name"), from))
      .unique()) as { balance: number; _id: Id<any> };
    const toDoc = (await db
      .query("accounts")
      .filter((q) => q.eq(q.field("name"), to))
      .unique()) as { balance: number; _id: Id<any> };

    if (fromDoc.balance < amount) {
      throw new Error("Insufficient balance");
    }
    fromDoc.balance -= amount;
    toDoc.balance += amount;

    await db.replace(fromDoc._id, fromDoc);
    await db.replace(toDoc._id, toDoc);
  },
});

export const fail = query({
  handler: (_, { i }: { i: number }) => {
    const messages = [
      "I can't go for that",
      "No can do.",
      "I'd do anything for love",
      "But I won't do that.",
    ];
    throw new Error(messages[i]);
  },
});

export const succeed = query({
  args: {},
  handler: () => {
    return "on my list";
  },
});

export const discardQueryResults = query({
  handler: async ({ db }, { throwError }: { throwError: boolean }) => {
    await db.query("accounts").collect();
    if (throwError) {
      throw new Error("bye");
    }
    return "hi";
  },
});
