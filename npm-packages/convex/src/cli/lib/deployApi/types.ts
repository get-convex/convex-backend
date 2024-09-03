import { z } from "zod";
import { looseObject } from "./utils.js";

export const reference = z.string();
export type Reference = z.infer<typeof reference>;

export const authInfo = looseObject({
  applicationID: z.string(),
  domain: z.string(),
});
export type AuthInfo = z.infer<typeof authInfo>;

export const identifier = z.string();
export type Identifier = z.infer<typeof identifier>;
