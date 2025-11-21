import { MutationCtx, internalMutation } from "./_generated/server";
import { internal } from "./_generated/api";
import { TableNames } from "./_generated/dataModel";
import { faker } from "@faker-js/faker";
import { didConvexYield } from "./yield";

export const generateUsers = internalMutation({
  args: {},
  handler: async ({ db }) => {
    if (await didConvexYield(db)) {
      return;
    }

    faker.seed();

    for (let i = 0; i < 1000; i++) {
      await db.insert("users", {
        name: faker.person.fullName(),
        userName: faker.internet.userName(),
      });
    }
  },
});

export const generateMessages = internalMutation({
  args: {},
  handler: async ({ db }) => {
    if (await didConvexYield(db)) {
      return;
    }

    faker.seed();

    const users = await db.query("users").take(100);
    if (users.length === 0) {
      console.debug("No users, not inserting messages");
      return;
    }

    for (let i = 0; i < 1000; i++) {
      await db.insert("messages", {
        body: faker.word.words(25),
        user: users[i % users.length]._id,
      });
    }
  },
});

type CleanTableArgs = {
  cursor: string | null;
  timestamp: number | null;
  table: TableNames;
};

export const cleanTable = internalMutation({
  handler: async (
    { db, scheduler }: MutationCtx,
    { cursor, timestamp, table }: CleanTableArgs,
  ) => {
    if (await didConvexYield(db)) {
      return;
    }
    if (timestamp === null) {
      const now = Date.now();
      const anHourAgo = new Date(now);
      anHourAgo.setHours(anHourAgo.getHours() - 1);
      timestamp = anHourAgo.getTime();
    }

    console.log(`Deleting items older than ${timestamp} from ${table}`);

    const { page, isDone, continueCursor } = await db
      .query(table)
      .withIndex("by_creation_time", (q) =>
        q.lt("_creationTime", timestamp as number),
      )
      .paginate({
        numItems: 1000,
        cursor,
      });

    console.log(`Deleting ${page.length} items, have more: ${!isDone}`);

    const deletePromises = [];
    for (const item of page) {
      deletePromises.push(db.delete(item._id));
    }
    await Promise.all(deletePromises);

    if (!isDone) {
      scheduler.runAfter(0, internal.generateDeleteData.cleanTable, {
        cursor: continueCursor,
        timestamp,
        table,
      });
    }
  },
});
