// This test exists to check that test in the convex directory work.

import { populate } from "./foods";

describe("foods", () => {
  test("foods action exists", async () => {
    expect(populate).toBeDefined();
  });
});
