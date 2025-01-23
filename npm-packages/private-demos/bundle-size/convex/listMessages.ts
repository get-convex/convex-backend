import { query } from "./_generated/server";
import { Message } from "../src/common";

// List all chat messages.
export default query(async ({ db }): Promise<Message[]> => {
  return await db.query("messages").collect();
});
