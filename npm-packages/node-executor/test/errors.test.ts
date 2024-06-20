import { extractErrorMessage } from "../src/errors";
import { describe, test, expect } from "vitest";

describe("ConvexHttpClient", () => {
  test("error object", () => {
    expect(extractErrorMessage(new Error("My error"))).toStrictEqual(
      "My error",
    );
  });

  test("no .message", () => {
    expect(extractErrorMessage(42)).toEqual("42");
    expect(extractErrorMessage("abracadabra")).toEqual("abracadabra");
    expect(extractErrorMessage({ value: 17 })).toStrictEqual("[object Object]");
  });

  test("falsy", () => {
    expect(extractErrorMessage(undefined)).toEqual("unknown error");
    expect(extractErrorMessage(null)).toEqual("unknown error");
    expect(extractErrorMessage(0)).toEqual("0");
    expect(extractErrorMessage("")).toEqual("");
  });

  test("malicious toString()", () => {
    expect(extractErrorMessage({ toString: 179 })).toEqual("unknown error");
    class NastyError1 {
      toString() {
        return 179;
      }
    }
    expect(extractErrorMessage(new NastyError1())).toEqual("unknown error");
    class NastyError2 {
      toString() {
        throw "Muhaha";
      }
    }
    expect(extractErrorMessage(new NastyError2())).toEqual("unknown error");
  });
});
