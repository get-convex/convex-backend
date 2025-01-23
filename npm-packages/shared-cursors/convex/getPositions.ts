import { query } from "./_generated/server";
import { Position } from "../src/types";

export default query(async ({ db }, { nonce = "" }): Promise<Position[]> => {
  // The nonce keeps queries from being deduped on the client and from being
  // cached on the server.
  console.log("positions for", nonce);
  const positions: Position[] = await db.query("positions").collect();
  return positions;
});
