import { NextApiRequest, NextApiResponse } from "next";
import { refreshSession } from "server/workos";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "POST") {
    res.setHeader("Allow", "POST");
    res.status(405).end("Method Not Allowed");
    return;
  }

  try {
    const result = await refreshSession(req);

    if (!result) {
      return res.status(401).json({ error: "No valid session found" });
    }

    res.status(200).json({ accessToken: result.accessToken });
  } catch (error: any) {
    console.error("Error refreshing session:", error);
    res.status(500).json({ error: error.message || "Internal server error" });
  }
}
