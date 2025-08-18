import { z } from "zod";
import { captureException, captureMessage } from "@sentry/nextjs";
import type { NextApiRequest, NextApiResponse } from "next";
import { getSession } from "server/workos";

export type ResponseData = {
  error: string | null;
};

const RequestBodySchema = z.object({
  subject: z.string(),
  message: z.string(),
  teamId: z.number(),
  projectId: z.number().optional(),
  deploymentName: z.string().optional(),
});

const UserSchema = z.object({
  email: z.string(),
  email_verified: z.boolean(),
  name: z.string().optional(),
  nickname: z.string().optional(),
});

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse<ResponseData>,
) {
  const session = await getSession(req);
  if (!session) {
    captureMessage("No session found");
    return res.status(401).json({ error: "Unauthorized" });
  }

  const { user } = session;

  let validatedUser: z.infer<typeof UserSchema>;
  try {
    validatedUser = UserSchema.parse(user);
  } catch (error: any) {
    captureException(error);
    return res.status(500).json({ error: "Internal Server Error" });
  }

  let body: z.infer<typeof RequestBodySchema>;
  try {
    body = RequestBodySchema.parse(req.body);
  } catch (error: any) {
    return res.status(400).json({ error: error.message });
  }

  try {
    // Get the host from the request headers
    const protocol = req.headers["x-forwarded-proto"] || "http";
    const host = req.headers["x-forwarded-host"] || req.headers.host;
    const baseUrl = `${protocol}://${host}`;

    void fetch(`${baseUrl}/api/send-plain-message`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Plain-Api-Key": process.env.PLAIN_API_KEY || "",
        "X-Convex-Access-Token": session.accessToken || "",
      },
      body: JSON.stringify({
        ...body,
        user: validatedUser,
      }),
    });

    res.status(200).json({ error: null });
  } catch (error: any) {
    captureException(error);
    return res.status(500).json({ error: "Internal Server Error" });
  }
}
