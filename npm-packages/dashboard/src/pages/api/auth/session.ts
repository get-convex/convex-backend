import { NextApiRequest, NextApiResponse } from "next";
import { getSession } from "server/workos";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "GET") {
    return res.status(405).json({ error: "Method not allowed" });
  }

  try {
    const session = await getSession(req);

    if (!session) {
      return res.status(401).json({ error: "Not authenticated" });
    }

    res.status(200).json(session);
  } catch (error) {
    console.error("Error getting session:", error);
    res.status(500).json({ error: "Internal server error" });
  }
}
