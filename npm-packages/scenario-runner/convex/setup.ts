import randomWords from "random-words";
import { EMBEDDING_SIZE, MESSAGES_TABLE, OPENCLAURD_TABLE } from "../types";
import { mutation } from "./_generated/server";
import { v } from "convex/values";
import { rand } from "./common";

export const setupMessages = mutation({
  handler: async (
    { db },
    { rows, channel }: { rows: number; channel: string },
  ): Promise<void> => {
    console.log(`Fill the messages table with ${rows} rows`);
    for (let i = 0; i < rows; i++) {
      // The first record always has rand=0. This is used in update_first
      // scenario.
      let randomNumber = 0;
      if (i > 0) {
        randomNumber = rand();
        if (i % 100 === 0) {
          console.log(`Filled ${i} messages so far`);
        }
      }
      const message = {
        channel,
        timestamp: 0,
        rand: randomNumber,
        ballastArray: [],
        body: randomWords({ exactly: 1 }).join(" "),
      };
      await db.insert(MESSAGES_TABLE, message);
    }
  },
});

export const setupVectors = mutation({
  args: {
    rows: v.number(),
  },
  handler: async (ctx, args) => {
    console.log(`Fill the vector table with ${args.rows} rows`);
    for (let i = 0; i < args.rows; i++) {
      const rand = Math.floor(Math.random() * 1000000000);
      if (i !== 0 && i % 100 === 0) {
        console.log(`Filled ${i} vector rows so far`);
      }

      const embedding = [];
      for (let j = 0; j < EMBEDDING_SIZE; j++) {
        embedding.push(Math.floor(Math.random() * 1000000000));
      }
      const openclaurd = {
        user: randomWords({ exactly: 1 }).join(" "),
        text: randomWords({ exactly: 10 }).join(" "),
        timestamp: 0,
        rand,
        embedding,
      };

      await ctx.db.insert(OPENCLAURD_TABLE, openclaurd);
    }
  },
});
