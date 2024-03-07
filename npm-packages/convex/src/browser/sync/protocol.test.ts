/**
 * @jest-environment jsdom
 */

import { test, expect } from "@jest/globals";

import { Long } from "../long.js";
import { longToU64, u64ToLong } from "./protocol.js";

test("Long serialization", async () => {
  expect(Long.fromNumber(89234097497)).toEqual(
    u64ToLong(longToU64(Long.fromNumber(89234097497))),
  );
});
