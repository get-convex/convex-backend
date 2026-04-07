import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";

// If modules have already been imported then custom condition
// type module resolution will work.
// https://github.com/kulshekhar/ts-jest/issues/4639

test("Warming up jest / ts-jest cache", async () => {
  const _ = ConvexHttpClient.toString() + ConvexReactClient.toString();
});
