import { joinUrlPath } from "@common/lib/helpers/joinUrlPath";

describe("joinUrlPath", () => {
  test("keeps path prefix when baseUrl ends with /", () => {
    expect(
      joinUrlPath(
        "http://localhost/convex/",
        "/api/check_admin_key",
      ).toString(),
    ).toBe("http://localhost/convex/api/check_admin_key");
  });

  test("keeps path prefix when baseUrl does not end with /", () => {
    expect(
      joinUrlPath("http://localhost/convex", "/api/check_admin_key").toString(),
    ).toBe("http://localhost/convex/api/check_admin_key");
  });

  test("accepts a path without a leading /", () => {
    expect(
      joinUrlPath("http://localhost/convex", "api/check_admin_key").toString(),
    ).toBe("http://localhost/convex/api/check_admin_key");
  });

  test("joins relative to the root when baseUrl has no prefix", () => {
    expect(
      joinUrlPath("http://localhost/", "/api/check_admin_key").toString(),
    ).toBe("http://localhost/api/check_admin_key");
  });

  test("joins relative to the root when baseUrl has no path", () => {
    expect(
      joinUrlPath("http://localhost", "/api/check_admin_key").toString(),
    ).toBe("http://localhost/api/check_admin_key");
  });

  test("preserves port and nested path prefixes", () => {
    expect(
      joinUrlPath(
        "https://example.com:1234/abc/def/",
        "/api/check_admin_key",
      ).toString(),
    ).toBe("https://example.com:1234/abc/def/api/check_admin_key");
  });
});
