import { z } from "zod";

export const reference = z.string();
export type Reference = z.infer<typeof reference>;

// passthrough for backward compat, auth was passthrough before custom JWTs were added
const Oidc = z
  .object({
    applicationID: z.string(),
    domain: z.string(),
  })
  .passthrough();

const CustomJwt = z.object({
  type: z.literal("customJwt"),
  applicationID: z.optional(z.string()),
  issuer: z.string(),
  jwks: z.string(),
  algorithm: z.string(),
});

export const authInfo = z.union([Oidc, CustomJwt]);

export type AuthInfo = z.infer<typeof authInfo>;

export const identifier = z.string();
export type Identifier = z.infer<typeof identifier>;
