import type { NextApiRequest, NextApiResponse } from "next";

type ConvexStatusIndicator = "none" | "minor" | "major" | "critical";

interface ConvexStatusResponse {
  status: {
    indicator: ConvexStatusIndicator;
    description: string;
  };
}

export default async function handler(
  request: NextApiRequest,
  response: NextApiResponse<ConvexStatusResponse | { error: string }>,
) {
  try {
    const statusResponse = await fetch(
      "https://status.convex.dev/api/v2/status.json",
      {
        method: "GET",
      },
    );

    if (!statusResponse.ok) {
      response.status(500).json({ error: "Failed to fetch Convex status" });
      return;
    }

    const statusData: ConvexStatusResponse = await statusResponse.json();

    response.status(200).json(statusData);
  } catch (error) {
    console.error("Error fetching Convex status:", error);
    response.status(500).json({ error: "Failed to fetch Convex status" });
  }
}
