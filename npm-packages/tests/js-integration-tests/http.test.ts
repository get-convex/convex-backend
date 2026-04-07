import { siteUrl } from "./common";

describe("HTTP actions", () => {
  test("custom authorization header", async () => {
    const url = `${siteUrl}/authHeader`;
    const response = await fetch(url, {
      method: "GET",
      headers: {
        Authorization: "Bearer helloworld",
      },
    });
    expect(response.status).toEqual(200);
    const result = await response.json();

    // The request should go through, propagating the header value.
    // But `auth.getUserIdentity` will return null since this header does not
    // correspond to a Convex managed auth provider
    expect(result.authorizationHeader).toEqual("Bearer helloworld");
    expect(result.identity).toEqual("error");
  });

  test("short custom authorization header", async () => {
    const url = `${siteUrl}/authHeader`;
    const response = await fetch(url, {
      method: "GET",
      headers: {
        Authorization: "a",
      },
    });
    expect(response.status).toEqual(200);
    const result = await response.json();

    // The request should go through, propagating the header value.
    // But `auth.getUserIdentity` will return null since this header does not
    // correspond to a Convex managed auth provider
    expect(result.authorizationHeader).toEqual("a");
    expect(result.identity).toEqual("error");
  });

  // TODO: add a test for a header value for a Convex managed auth provider.

  test("root path is accessible", async () => {
    const url = `${siteUrl}/`;
    const response = await fetch(url, {
      method: "POST",
      body: "Hello world",
    });
    expect(response.ok).toEqual(true);
    const result = await response.text();

    expect(result).toEqual("Hello world");
  });

  test("http action failure", async () => {
    const url = `${siteUrl}/failer`;
    const response = await fetch(url);
    expect(response.ok).toEqual(false);
    const result = await response.json();
    expect(result.code).toContain("Uncaught Error: ErrMsg");
    expect(result.trace).toMatch(/^Uncaught Error: ErrMsg/);
    expect(result.trace).toContain("at <anonymous> (../convex/http.ts:");
  });
});
