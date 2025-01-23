import { mutation } from "./_generated/server";

// Send a chat message.
export default mutation(
  ({ db }, { body, author }: { body: string; author: string }) => {
    const message = { body, author };
    return db.insert("messages", message);
  },
);
