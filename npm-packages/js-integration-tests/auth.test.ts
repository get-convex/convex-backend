import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { opts } from "./test_helpers";
import fs from "fs";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

// From admin_key.txt
const adminKey = fs.readFileSync(
  "../../crates/keybroker/dev/admin_key.txt",
  "utf8",
);

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
