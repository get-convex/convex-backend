import {mutation} from './_generated/server';

export default mutation({
  handler: async ({ db }, { body, author }) => {
    const message = { body, author };
    await db.insert('messages', message);
  }
});