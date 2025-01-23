import type { NextApiRequest, NextApiResponse } from "next";
import { fetchQuery } from "convex/nextjs";
import { api } from "../../convex/_generated/api";

export const count = async function handler(
  _req: NextApiRequest,
  res: NextApiResponse,
) {
  const clicks = await fetchQuery(api.counter.get, { counterName: "clicks" });
  res.status(200).json({ clicks });
};
