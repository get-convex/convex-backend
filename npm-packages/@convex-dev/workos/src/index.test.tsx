import { test } from "vitest";
import { ConvexProviderWithAuthKit } from "./index.js";
import { ConvexReactClient } from "convex/react";
import { useAuth } from "@workos-inc/authkit-react";

test("Helpers are valid children", () => {
  const convex = new ConvexReactClient("https://localhost:3001");

  const _ = (
    <ConvexProviderWithAuthKit client={convex} useAuth={useAuth}>
      Hello world
    </ConvexProviderWithAuthKit>
  );
});
