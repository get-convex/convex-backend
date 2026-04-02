import { describe, test, expect } from "vitest";
import {
  parseExpiration,
  resolveExpiration,
  validateExpiration,
} from "./expiration.js";

describe("parseExpiration", () => {
  test('"none" returns kind "none"', () => {
    expect(parseExpiration("none")).toEqual({ kind: "none" });
  });

  test("Unix seconds (< 1e12) converts to ms", () => {
    const result = parseExpiration("1711828382");
    expect(result).toEqual({ kind: "absolute", timestampMs: 1711828382000 });
  });

  test("Unix milliseconds (>= 1e12) used as-is", () => {
    const result = parseExpiration("1711828382000");
    expect(result).toEqual({ kind: "absolute", timestampMs: 1711828382000 });
  });

  test("UTC datetime", () => {
    const result = parseExpiration("2026-03-30T18:53:02Z");
    expect(result).toEqual({
      kind: "absolute",
      timestampMs: new Date("2026-03-30T18:53:02Z").getTime(),
    });
  });

  test("rejects datetime without Z suffix", () => {
    const result = parseExpiration("2026-03-30T18:53:02");
    expect(result.kind).toBe("error");
  });

  test("rejects datetime with timezone offset", () => {
    const result = parseExpiration("2026-03-30T18:53:02+01:00");
    expect(result.kind).toBe("error");
  });

  test("rejects bare date without time", () => {
    const result = parseExpiration("2026-03-30");
    expect(result.kind).toBe("error");
  });

  test('relative: "in 1 hour"', () => {
    expect(parseExpiration("in 1 hour")).toEqual({
      kind: "relative",
      amount: 1,
      unit: "hour",
    });
  });

  test('relative: "in 3 hours"', () => {
    expect(parseExpiration("in 3 hours")).toEqual({
      kind: "relative",
      amount: 3,
      unit: "hour",
    });
  });

  test('relative: "in 7 days"', () => {
    expect(parseExpiration("in 7 days")).toEqual({
      kind: "relative",
      amount: 7,
      unit: "day",
    });
  });

  test('relative: "in 45 minutes"', () => {
    expect(parseExpiration("in 45 minutes")).toEqual({
      kind: "relative",
      amount: 45,
      unit: "minute",
    });
  });

  test("invalid input returns error", () => {
    const result = parseExpiration("yesterday");
    expect(result).toEqual({
      kind: "error",
      message: expect.stringContaining("Invalid expiration format"),
    });
  });

  test("empty string returns error", () => {
    const result = parseExpiration("");
    expect(result).toEqual({
      kind: "error",
      message: expect.stringContaining("Invalid expiration format"),
    });
  });
});

describe("resolveExpiration", () => {
  const now = 1700000000000;

  test('"none" resolves to null', () => {
    expect(resolveExpiration({ kind: "none" }, now)).toBeNull();
  });

  test("absolute resolves to timestampMs", () => {
    expect(
      resolveExpiration({ kind: "absolute", timestampMs: 1711828382000 }, now),
    ).toBe(1711828382000);
  });

  test("relative hours", () => {
    expect(
      resolveExpiration({ kind: "relative", amount: 3, unit: "hour" }, now),
    ).toBe(now + 3 * 60 * 60 * 1000);
  });

  test("relative days", () => {
    expect(
      resolveExpiration({ kind: "relative", amount: 7, unit: "day" }, now),
    ).toBe(now + 7 * 24 * 60 * 60 * 1000);
  });

  test("relative minutes", () => {
    expect(
      resolveExpiration({ kind: "relative", amount: 45, unit: "minute" }, now),
    ).toBe(now + 45 * 60 * 1000);
  });
});

describe("validateExpiration", () => {
  const now = 1700000000000;

  test("past timestamp returns error", () => {
    const result = validateExpiration(now - 1000, now);
    expect(result).toEqual({
      kind: "error",
      message: expect.stringContaining("in the future"),
    });
  });

  test("less than 30 minutes from now returns error", () => {
    const result = validateExpiration(now + 10 * 60 * 1000, now);
    expect(result).toEqual({
      kind: "error",
      message: expect.stringContaining("at least 30 minutes"),
    });
  });

  test("more than 1 year from now returns error", () => {
    const overOneYear = now + 366 * 24 * 60 * 60 * 1000;
    const result = validateExpiration(overOneYear, now);
    expect(result).toEqual({
      kind: "error",
      message: expect.stringContaining("at most 1 year"),
    });
  });

  test("valid timestamp returns success", () => {
    const twoHours = now + 2 * 60 * 60 * 1000;
    expect(validateExpiration(twoHours, now)).toEqual({ kind: "success" });
  });
});
