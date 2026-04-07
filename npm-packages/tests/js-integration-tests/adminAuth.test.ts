import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { opts } from "./test_helpers";
import { api } from "./convex/_generated/api";
import { adminKey, deploymentUrl } from "./common";

describe("auth acting as user", () => {
  test("http client", async () => {
    const httpClient = new ConvexHttpClient(deploymentUrl);
    httpClient.setAdminAuth(adminKey, {
      subject: "test subject",
      issuer: "test issuer",
      name: "Presley",
    });
    const result = await httpClient.query(api.auth.q);
    expect(result?.name).toEqual("Presley");
  });

  test("react client", async () => {
    const reactClient = new ConvexReactClient(deploymentUrl, opts);
    reactClient.setAdminAuth(adminKey, {
      subject: "test subject",
      issuer: "test issuer",
      name: "Presley",
    });
    const result = await reactClient.query(api.auth.q);
    expect(result?.name).toEqual("Presley");
    await reactClient.close();
  });

  test("http client utf16", async () => {
    const httpClient = new ConvexHttpClient(deploymentUrl);
    httpClient.setAdminAuth(adminKey, {
      subject: "test subject",
      issuer: "test issuer",
      name: "Presley 🙃",
    });
    const result = await httpClient.query(api.auth.q);
    expect(result?.name).toEqual("Presley 🙃");
  });

  test("react client utf16", async () => {
    const reactClient = new ConvexReactClient(deploymentUrl, opts);
    reactClient.setAdminAuth(adminKey, {
      subject: "test subject",
      issuer: "test issuer",
      name: "Presley 🙃",
    });
    const result = await reactClient.query(api.auth.q);
    expect(result?.name).toEqual("Presley 🙃");
    await reactClient.close();
  });
});
