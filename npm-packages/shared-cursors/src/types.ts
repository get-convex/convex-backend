export type Position = {
  x: number; // x and y are prep for a future functionality we actually share
  y: number; // cursor position
  clientSentTs: number; // `Date.now()` on the client that made a mutation
  serverSentTs: number; // server `Date.now()` in that mutation
  session: string; // identity of the client
};
