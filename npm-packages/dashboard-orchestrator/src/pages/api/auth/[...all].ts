// BetterAuth catch-all route. Handles /api/auth/sign-in/email,
// /api/auth/sign-up/email, /api/auth/sign-out, /api/auth/get-session,
// /api/auth/callback/{github,google,...}, /api/auth/verify-email, etc.

import { toNodeHandler } from "better-auth/node";
import type { NextApiRequest, NextApiResponse } from "next";
import { auth } from "../../../lib/auth";

export const config = {
  api: { bodyParser: false },
};

const handler = toNodeHandler(auth);

export default function authHandler(req: NextApiRequest, res: NextApiResponse) {
  return handler(req, res);
}
