// If multiple pages components need to use the client, define it in a separate file like this.

import { ConvexReactClient } from "convex/react";
import process from "process";

export const convex = new ConvexReactClient(
  process.env.NEXT_PUBLIC_CONVEX_URL!,
);
