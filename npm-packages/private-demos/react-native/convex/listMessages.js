import {query} from './_generated/server';

export default query({
  handler: async ({ db }) => {
    return await db.query('messages').collect();
  }
});