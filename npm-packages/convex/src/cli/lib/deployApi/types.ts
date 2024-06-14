import { z } from "zod";

export const reference = z.string();
export type Reference = z.infer<typeof reference>;

export const authInfo = z.object({
  applicationID: z.string(),
  domain: z.string(),
});
export type AuthInfo = z.infer<typeof authInfo>;

export const identifier = z.string();
export type Identifier = z.infer<typeof identifier>;
