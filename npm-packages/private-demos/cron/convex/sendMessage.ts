import { mutation } from "./_generated/server";

const foo = mutation(
  async ({ db }, { body, author }: { body: string; author: string }) => {
    const message = { body, author };
    await db.insert("messages", message);
  },
);

export default foo;
