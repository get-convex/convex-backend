import { decodeId } from "./index.js";
import { describe, expect, test } from "@jest/globals";

describe("Id encoding tests", () => {
  test("Document ID stability", () => {
    const parsedId = decodeId("z43zp6c3e75gkmz1kfwj6mbbx5sw281h");
    const expected = Array(16).fill(251);
    for (let i = 1; i < 16; i++) {
      expected[i] = (expected[i - 1] * 251) % 256;
    }
    expect([...parsedId.internalId.values()]).toEqual(expected);
    expect(parsedId.tableNumber).toBe(1017);
  });
});
