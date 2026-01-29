import { test, expect, describe } from "vitest";
import {
  getConvexSize,
  getDocumentSize,
  SYSTEM_FIELD_CREATION_TIME_SIZE,
  SYSTEM_FIELD_ID_ESTIMATE,
} from "./size.js";

/**
 * Tests for getConvexSize JS-specific behavior.
 * Cross-language consistency with Rust is tested via proptests in
 * crates/isolate/src/tests/values.rs which call the JS implementation.
 */
describe("getConvexSize", () => {
  test("long strings use TextEncoder path (>500 chars)", () => {
    // This tests the JS-specific optimization for long strings
    const longAscii = "a".repeat(1000);
    expect(getConvexSize(longAscii)).toBe(1002); // 1000 + 2 overhead

    const longUtf8 = "héllo".repeat(200); // 6 bytes each
    expect(getConvexSize(longUtf8)).toBe(1202); // 1200 + 2 overhead
  });
});

/**
 * Tests for getDocumentSize - the system fields handling logic.
 */
describe("getDocumentSize", () => {
  test("adds system fields overhead when not present", () => {
    const doc = { name: "Alice" };
    const baseSize = getConvexSize(doc);
    expect(getDocumentSize(doc)).toBe(
      baseSize + SYSTEM_FIELD_ID_ESTIMATE + SYSTEM_FIELD_CREATION_TIME_SIZE,
    );
  });

  test("empty document with system fields", () => {
    expect(getConvexSize({})).toBe(2);
    expect(getDocumentSize({})).toBe(
      2 + SYSTEM_FIELD_ID_ESTIMATE + SYSTEM_FIELD_CREATION_TIME_SIZE,
    );
  });

  test("does not double-count existing system fields", () => {
    const doc = {
      _id: "abc123def456ghi789jkl012mno345pq",
      _creationTime: 1234567890123,
      name: "Alice",
    };
    const baseSize = getConvexSize(doc);
    expect(getDocumentSize(doc)).toBe(baseSize);
  });

  test("handles partial system fields", () => {
    // Only _id present
    const docWithId = {
      _id: "abc123def456ghi789jkl012mno345pq",
      name: "Alice",
    };
    const sizeWithId = getConvexSize(docWithId);
    expect(getDocumentSize(docWithId)).toBe(
      sizeWithId + SYSTEM_FIELD_CREATION_TIME_SIZE,
    ); // adds _creationTime

    // Only _creationTime present
    const docWithTime = {
      _creationTime: 1234567890123,
      name: "Alice",
    };
    const sizeWithTime = getConvexSize(docWithTime);
    expect(getDocumentSize(docWithTime)).toBe(sizeWithTime + 38); // adds _id
  });

  test("customIdLength option", () => {
    const doc = { name: "Alice", _creationTime: 1234567890123 };
    const customIdLength = 40;

    // With custom ID length: 4 (field) + 2 (string overhead) + 40 = 46 bytes
    const withCustomId = getDocumentSize(doc, { customIdLength });

    // Verify by computing with actual _id of that length
    const _id = "a".repeat(customIdLength);
    const withActualId = getConvexSize({ ...doc, _id });
    expect(withCustomId).toBe(withActualId);
  });
});
