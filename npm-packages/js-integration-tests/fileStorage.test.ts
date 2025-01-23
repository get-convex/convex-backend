import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import { awaitQueryResult, opts } from "./test_helpers";
import { sha256 } from "js-sha256";
import { Id } from "./convex/_generated/dataModel";
import { version } from "convex";
import { deploymentUrl, siteUrl } from "./common";

function getDigestHeader(image: string) {
  const digest = sha256.array(image);
  const base64Digest = btoa(String.fromCharCode(...digest));
  const digestHeader = `sha-256=${base64Digest}`;
  return digestHeader;
}

describe("File storage with HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("file storage post", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const json = await postResult.json();
    const { storageId } = json;
    if (!storageId) {
      throw Error(
        `Failed to get storage id from ${postResult}, ${postResult.status}:
        ${postResult.statusText}`,
      );
    }

    const getUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });

    const getResult = await fetch(getUrl!);
    expect(getResult.headers.get("Content-Type")).toEqual("text/plain");
    expect(getResult.headers.get("Content-Length")).toEqual(
      "helloworld".length.toString(),
    );
    expect(getResult.headers.get("Cache-Control")).toEqual(
      "private, max-age=2592000",
    );
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("file storage empty file", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();
    if (!storageId) {
      throw Error(
        `Failed to get storage id from ${postResult}, ${postResult.status}:
        ${postResult.statusText}`,
      );
    }

    const getUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });

    const getResult = await fetch(getUrl!);
    expect(getResult.headers.get("Content-Type")).toEqual("text/plain");
    expect(getResult.headers.get("Content-Length")).toEqual("0");
    expect(getResult.headers.get("Cache-Control")).toEqual(
      "private, max-age=2592000",
    );
    expect(await getResult.text()).toEqual("");
  });

  test("file storage post returns new id always", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const json = await postResult.json();
    const { storageId } = json;
    if (!storageId) {
      throw Error(
        `Failed to get storage id from ${postResult}, ${postResult.status}:
        ${postResult.statusText}`,
      );
    }

    const digest = sha256.array("helloworld");
    const base64Digest = btoa(String.fromCharCode(...digest));
    const hexDigest = sha256("helloworld");

    const metadata = await httpClient.query(api.fileStorage.get, {
      id: storageId,
    });
    const deprecatedGetMetadata = await httpClient.query(
      api.fileStorage.getMetadata,
      {
        storageId,
      },
    );
    expect(metadata?.contentType).toEqual(deprecatedGetMetadata?.contentType);
    const [major, minor] = version.split(".");
    if (+major > 1 || +minor >= 9) {
      expect(metadata?.sha256).toEqual(base64Digest);
    } else {
      expect(metadata?.sha256).toEqual(hexDigest);
    }
    expect(metadata?.size).toEqual(deprecatedGetMetadata?.size);
  });

  test("file storage deletes", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "specialrelativity",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId }: { storageId: Id<"_storage"> } =
      await postResult.json();
    const getUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });

    const result1 = await httpClient.mutation(api.fileStorage.deleteById, {
      storageId,
    });
    expect(result1).toEqual(null);
    await expect(
      httpClient.mutation(api.fileStorage.deleteById, { storageId }),
    ).rejects.toThrow(/Uncaught Error: storage id \S+ not found/);
    await expect(
      httpClient.mutation(api.fileStorage.deleteById, {
        storageId: "nonsense",
      }),
    ).rejects.toThrow("Invalid argument `storageId` for `storage.delete`");

    // Trying to fetch a url after delete should fail
    const badUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });
    expect(badUrl).toEqual(null);

    // Trying to use a bad URL from before the delete should 404
    expect(getUrl).not.toEqual(null);
    const getResult = await fetch(getUrl!);
    expect(getResult.status).toEqual(404);
    expect((await getResult.json()).code).toEqual("FileNotFound");
  });

  test("file storage deletes with old id", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "specialrelativity",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId }: { storageId: Id<"_storage"> } =
      await postResult.json();
    const getUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });

    const metadata = await httpClient.query(api.fileStorage.getMetadata, {
      storageId,
    });
    expect(metadata).toHaveProperty("storageId");
    const oldStorageId = (metadata as any).storageId;
    const result = await httpClient.mutation(api.fileStorage.deleteById, {
      storageId: oldStorageId,
    });
    expect(result).toEqual(null);
    await expect(
      httpClient.mutation(api.fileStorage.deleteById, {
        storageId: oldStorageId,
      }),
    ).rejects.toThrow(/Uncaught Error: storage id \S+ not found/);
    await expect(
      httpClient.mutation(api.fileStorage.deleteById, {
        storageId: "nonsense",
      }),
    ).rejects.toThrow("Invalid argument `storageId` for `storage.delete`");

    // Trying to fetch a url after delete should fail
    const badUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId: oldStorageId,
    });
    expect(badUrl).toEqual(null);

    // Trying to use a bad URL from before the delete should 404
    expect(getUrl).not.toEqual(null);
    const getResult = await fetch(getUrl!);
    expect(getResult.status).toEqual(404);
    expect((await getResult.json()).code).toEqual("FileNotFound");
  });

  test("file storage getMetadata", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId }: { storageId: Id<"_storage"> } =
      await postResult.json();
    const metadata = await httpClient.query(api.fileStorage.getMetadata, {
      storageId,
    });
    expect(metadata).toEqual(
      expect.objectContaining({
        sha256: sha256("helloworld"),
        size: "helloworld".length,
        contentType: "text/plain",
      }),
    );

    // After deletion, it should return null
    await httpClient.mutation(api.fileStorage.deleteById, { storageId });
    const nullMetadata = await httpClient.query(api.fileStorage.getMetadata, {
      storageId,
    });
    expect(nullMetadata).toBeNull();
  });

  test("file storage getMetadata with old id", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId }: { storageId: Id<"_storage"> } =
      await postResult.json();
    const metadata = await httpClient.query(api.fileStorage.getMetadata, {
      storageId,
    });
    expect(metadata).toEqual(
      expect.objectContaining({
        sha256: sha256("helloworld"),
        size: "helloworld".length,
        contentType: "text/plain",
      }),
    );
    expect(metadata).toHaveProperty("storageId");
    const oldStorageId = (metadata as any).storageId;
    const deprecatedGetMetadata = await httpClient.query(
      api.fileStorage.getMetadata,
      {
        storageId: oldStorageId,
      },
    );
    expect(metadata).toEqual(deprecatedGetMetadata);
    // After deletion, it should return null
    await httpClient.mutation(api.fileStorage.deleteById, {
      storageId: oldStorageId,
    });
    const nullMetadata = await httpClient.query(api.fileStorage.getMetadata, {
      storageId: oldStorageId,
    });
    expect(nullMetadata).toBeNull();
  });

  test("upload succeeds with sha256", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const digestHeader = getDigestHeader("helloworld");
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
        Digest: digestHeader,
      },
    });
    expect(postResult.status).toEqual(200);
  });

  test("upload fails on sha256 mismatch", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const digestHeader = getDigestHeader("helloworld");
    const badPostResult = await fetch(postUrl, {
      method: "POST",
      body: "differentdata",
      headers: {
        "Content-Type": "text/plain",
        Digest: digestHeader,
      },
    });
    expect(badPostResult.status).toEqual(400);
  });

  test("expired upload url", async () => {
    const expiredPostUrl = `${deploymentUrl}/api/storage/upload?token=012df53dad7f240d44729afc7d018378f47916060026f5870066118050545b6f45f52467338e8bd2826bf358c80b7d7846a080719fa0fc0a2d69e7`;
    const postResult = await fetch(expiredPostUrl, {
      method: "POST",
      body: "helloworld",
    });
    expect(postResult.status).toEqual(401);
    expect((await postResult.json()).code).toEqual("StorageTokenExpired");
  });

  test("uploading invalid token", async () => {
    const expiredPostUrl = `${deploymentUrl}/api/storage/upload?token=coffee`;
    const postResult = await fetch(expiredPostUrl, {
      method: "POST",
      body: "helloworld",
    });
    expect(postResult.status).toEqual(401);
    expect((await postResult.json()).code).toEqual("StorageTokenInvalid");
  });

  test("HTTP GET range request", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    // smallest MP3
    const body = Buffer.from(
      "ffe318c40000000348000000004c414d45332e39382e3200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
      "hex",
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body,
      headers: { "Content-Type": "audio/mpeg" },
    });
    const { storageId } = await postResult.json();
    const getUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });
    const getResult = await fetch(getUrl!, {
      headers: {
        Range: "bytes=20-29",
      },
    });
    expect(getResult.status).toEqual(206);
    expect(getResult.headers.get("Content-Type")).toEqual("audio/mpeg");
    expect(getResult.headers.get("Content-Range")).toEqual(
      `bytes 20-29/${body.length}`,
    );
    expect(Buffer.from(await getResult.arrayBuffer())).toEqual(
      body.slice(20, 30),
    );
  });

  test("HTTP GET range request empty file", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "",
      headers: { "Content-Type": "text/plain" },
    });
    const { storageId } = await postResult.json();
    const getUrl = await httpClient.query(api.fileStorage.getImageUrl, {
      storageId,
    });
    const getResult = await fetch(getUrl!, {
      headers: {
        Range: "bytes=0-0",
      },
    });
    expect(getResult.status).toEqual(200);
    expect(getResult.headers.get("Content-Type")).toEqual("text/plain");
    expect(getResult.headers.get("Content-Range")).toEqual(null);
    expect(getResult.headers.get("Content-Length")).toEqual("0");
    expect(await getResult.text()).toEqual("");
  });
});

describe("File storage with ConvexReactClient", () => {
  let reactClient: ConvexReactClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  test("file storage post", async () => {
    const postUrl = await reactClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
    });
    const { storageId } = await postResult.json();

    const watch = reactClient.watchQuery(api.fileStorage.getImageUrl, {
      storageId,
    });
    const getUrl = await awaitQueryResult(watch, (url) => url !== undefined);

    const getResult = await fetch(getUrl!);
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("file storage post with old id", async () => {
    const postUrl = await reactClient.mutation(
      api.fileStorage.generateUploadUrl,
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
    });
    const { storageId } = await postResult.json();

    const watch1 = reactClient.watchQuery(api.fileStorage.getMetadata, {
      storageId,
    });
    const metadata = await awaitQueryResult(watch1, (doc) => doc !== undefined);
    expect(metadata).toHaveProperty("storageId");
    const oldStorageId = (metadata as any).storageId;

    const watch2 = reactClient.watchQuery(api.fileStorage.getImageUrl, {
      storageId: oldStorageId,
    });
    const getUrl = await awaitQueryResult(watch2, (url) => url !== undefined);

    const getResult = await fetch(getUrl!);
    expect(await getResult.text()).toEqual("helloworld");
  });
});

describe("File storage with HTTP actions", () => {
  const postUrl = `${siteUrl}/sendImage`;
  const getUrl = new URL(`${siteUrl}/getImage`);
  const getMetadata = new URL(`${siteUrl}/getMetadata`);
  const deleteUrl = new URL(`${siteUrl}/deleteImage`);
  test("file storage post", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();

    getUrl.searchParams.set("storageId", storageId);

    const getResult = await fetch(getUrl.href);
    expect(getResult.headers.get("Content-Type")).toEqual("text/plain");
    expect(getResult.headers.get("Content-Length")).toEqual(
      "helloworld".length.toString(),
    );
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("file storage post with old id", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();
    getMetadata.searchParams.set("storageId", storageId);
    const metadataResponse = await fetch(getMetadata.href);
    const metadata = JSON.parse(await metadataResponse.text());

    getUrl.searchParams.set("storageId", (metadata as any).storageId);

    const getResult = await fetch(getUrl.href);
    expect(getResult.headers.get("Content-Type")).toEqual("text/plain");
    expect(getResult.headers.get("Content-Length")).toEqual(
      "helloworld".length.toString(),
    );
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("file storage post with no content-type", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
    });
    const data = await postResult.json();
    expect(data).toHaveProperty("storageId");
  });

  test("file storage post with empty blob", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const data = await postResult.json();
    expect(data).toHaveProperty("storageId");
  });

  test("file storage post with empty blob and no content-type", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
    });
    const data = await postResult.json();
    expect(data).toHaveProperty("storageId");
  });

  test("file storage deletes", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();

    getUrl.searchParams.set("storageId", storageId);

    const getResult1 = await fetch(getUrl.href);
    expect(await getResult1.text()).toEqual("helloworld");

    deleteUrl.searchParams.set("storageId", storageId);

    const deleteResult1 = await fetch(deleteUrl.href, { method: "POST" });
    expect(deleteResult1.ok).toBe(true);

    // Getting a deleted image should 404
    const getResult2 = await fetch(getUrl.href);
    expect(getResult2.status).toEqual(404);

    // Deleting an image that no longer exists
    const deleteResult2 = await fetch(deleteUrl.href, { method: "POST" });
    const deleteResult2Text = await deleteResult2.text();
    expect(deleteResult2Text).toMatch(
      /Uncaught Error: storage id \S+ not found/,
    );

    // Passing a nonesense storage ID
    deleteUrl.searchParams.set("storageId", "nonesense");
    const deleteResult3 = await fetch(deleteUrl.href, { method: "POST" });
    const deleteResult3Text = await deleteResult3.text();
    expect(deleteResult3Text).toContain(
      "Invalid argument `storageId` for `storage.delete`",
    );
  });

  test("file storage deletes with old id", async () => {
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();
    getMetadata.searchParams.set("storageId", storageId);
    const metadataResponse = await fetch(getMetadata.href);
    const metadata = JSON.parse(await metadataResponse.text());
    const oldStorageId = (metadata as any).storageId;

    getUrl.searchParams.set("storageId", oldStorageId);

    const getResult1 = await fetch(getUrl.href);
    expect(await getResult1.text()).toEqual("helloworld");

    deleteUrl.searchParams.set("storageId", oldStorageId);

    const deleteResult1 = await fetch(deleteUrl.href, { method: "POST" });
    expect(deleteResult1.ok).toBe(true);

    // Getting a deleted image should 404
    const getResult2 = await fetch(getUrl.href);
    expect(getResult2.status).toEqual(404);

    // Deleting an image that no longer exists
    const deleteResult2 = await fetch(deleteUrl.href, { method: "POST" });
    const deleteResult2Text = await deleteResult2.text();
    expect(deleteResult2Text).toMatch(
      /Uncaught Error: storage id \S+ not found/,
    );

    // Passing a nonesense storage ID
    deleteUrl.searchParams.set("storageId", "nonesense");
    const deleteResult3 = await fetch(deleteUrl.href, { method: "POST" });
    const deleteResult3Text = await deleteResult3.text();
    expect(deleteResult3Text).toContain(
      "Invalid argument `storageId` for `storage.delete`",
    );
  });

  test("upload fails on sha256 mismatch", async () => {
    const digestHeader = getDigestHeader("helloworld");
    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        "Content-Type": "text/plain",
        Digest: digestHeader,
      },
    });
    expect(postResult.status).toEqual(200);

    const badPostResult = await fetch(postUrl, {
      method: "POST",
      body: "differentdata",
      headers: {
        "Content-Type": "text/plain",
        Digest: digestHeader,
      },
    });
    expect(badPostResult.status).toEqual(500);
  });
});

describe("File storage with V8 actions", () => {
  let reactClient: ConvexReactClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  // Skipped while Tom figure this out
  // eslint-disable-next-line jest/no-disabled-tests
  test.skip("generateUploadUrl and getUrl", async () => {
    const postUrl = await reactClient.action(
      api.fileStorageV8Actions.generateUploadUrl,
      {},
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        Digest: getDigestHeader("helloworld"),
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();

    const getUrl = await reactClient.action(api.fileStorageV8Actions.getUrl, {
      storageId,
    });
    const getResult = await fetch(getUrl!);
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("store and get", async () => {
    const storageId = await reactClient.action(api.fileStorageV8Actions.store, {
      content: "helloworld",
      contentType: "text/plain",
    });
    const getResult = await reactClient.action(api.fileStorageV8Actions.get, {
      storageId: storageId,
    });
    expect(getResult).toEqual("helloworld");
  });

  test("store then get using old id", async () => {
    const storageId = await reactClient.action(api.fileStorageV8Actions.store, {
      content: "helloworld",
      contentType: "text/plain",
    });
    const metadata = await reactClient.action(
      api.fileStorageV8Actions.getMetadata,
      {
        storageId: storageId,
      },
    );
    const oldStorageId = (metadata as any).storageId;
    const getResult = await reactClient.action(api.fileStorageV8Actions.get, {
      storageId: oldStorageId,
    });
    expect(getResult).toEqual("helloworld");
  });

  test("deletes", async () => {
    const storageId = await reactClient.action(api.fileStorageV8Actions.store, {
      content: "helloworld",
      contentType: "text/plain",
    });
    const deleteResult = await reactClient.action(
      api.fileStorageV8Actions.deleteById,
      {
        storageId,
      },
    );
    expect(deleteResult).toEqual(null);
    const getUrl = await reactClient.action(api.fileStorageV8Actions.getUrl, {
      storageId,
    });
    expect(getUrl).toEqual(null);
    const getResult = await reactClient.action(api.fileStorageV8Actions.get, {
      storageId: storageId,
    });
    expect(getResult).toEqual(null);
  });

  test("deletes with old id", async () => {
    const storageId = await reactClient.action(api.fileStorageV8Actions.store, {
      content: "helloworld",
      contentType: "text/plain",
    });
    const metadata = await reactClient.action(
      api.fileStorageV8Actions.getMetadata,
      {
        storageId: storageId,
      },
    );
    const oldStorageId = (metadata as any).storageId;
    const deleteResult = await reactClient.action(
      api.fileStorageV8Actions.deleteById,
      {
        storageId: oldStorageId,
      },
    );
    expect(deleteResult).toEqual(null);
    const getUrl = await reactClient.action(api.fileStorageV8Actions.getUrl, {
      storageId: oldStorageId,
    });
    expect(getUrl).toEqual(null);
    const getResult = await reactClient.action(api.fileStorageV8Actions.get, {
      storageId: oldStorageId,
    });
    expect(getResult).toEqual(null);
  });

  test("getMetadata", async () => {
    const storageId = await reactClient.action(api.fileStorageV8Actions.store, {
      content: "helloworld",
      contentType: "text/plain",
    });
    const metadata = await reactClient.action(
      api.fileStorageV8Actions.getMetadata,
      { storageId: storageId },
    );
    expect(metadata).toEqual(
      expect.objectContaining({
        sha256: sha256("helloworld"),
        size: "helloworld".length,
        contentType: "text/plain",
      }),
    );
  });

  test("getMetadata with old id", async () => {
    const storageId = await reactClient.action(api.fileStorageV8Actions.store, {
      content: "helloworld",
      contentType: "text/plain",
    });
    const metadata = await reactClient.action(
      api.fileStorageV8Actions.getMetadata,
      { storageId: storageId },
    );

    const oldStorageId = (metadata as any).storageId;
    const metadata2 = await reactClient.action(
      api.fileStorageV8Actions.getMetadata,
      { storageId: oldStorageId },
    );
    expect(metadata2).toEqual({
      storageId: oldStorageId,
      sha256: sha256("helloworld"),
      size: "helloworld".length,
      contentType: "text/plain",
    });
  });
});

describe("File storage with Node actions", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("generateUploadUrl and getUrl", async () => {
    const postUrl = await httpClient.action(
      api.fileStorageNodeActions.generateUploadUrl,
      {},
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        Digest: getDigestHeader("helloworld"),
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();

    const getUrl = await httpClient.action(api.fileStorageNodeActions.getUrl, {
      storageId,
    });
    const getResult = await fetch(getUrl!);
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("generateUploadUrl and getUrl with old id", async () => {
    const postUrl = await httpClient.action(
      api.fileStorageNodeActions.generateUploadUrl,
      {},
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    const postResult = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        Digest: getDigestHeader("helloworld"),
        "Content-Type": "text/plain",
      },
    });
    const { storageId } = await postResult.json();
    const metadata = await httpClient.action(
      api.fileStorageNodeActions.getMetadata,
      {
        storageId,
      },
    );
    const oldStorageId = (metadata as any).storageId;

    const getUrl = await httpClient.action(api.fileStorageNodeActions.getUrl, {
      storageId: oldStorageId,
    });
    const getResult = await fetch(getUrl!);
    expect(await getResult.text()).toEqual("helloworld");
  });

  test("store and get", async () => {
    const storageId = await httpClient.action(
      api.fileStorageNodeActions.store,
      {
        content: "helloworld",
        contentType: "text/plain",
      },
    );
    const getResult = await httpClient.action(api.fileStorageNodeActions.get, {
      storageId: storageId,
    });
    expect(getResult).toEqual("helloworld");
  });

  test("store and get with old id", async () => {
    const storageId = await httpClient.action(
      api.fileStorageNodeActions.store,
      {
        content: "helloworld",
        contentType: "text/plain",
      },
    );
    const metadata = await httpClient.action(
      api.fileStorageNodeActions.getMetadata,
      {
        storageId,
      },
    );
    const oldStorageId = (metadata as any).storageId;

    const getResult = await httpClient.action(api.fileStorageNodeActions.get, {
      storageId: oldStorageId,
    });
    expect(getResult).toEqual("helloworld");
  });

  test("deletes", async () => {
    const storageId = await httpClient.action(
      api.fileStorageNodeActions.store,
      {
        content: "helloworld",
        contentType: "text/plain",
      },
    );
    const deleteResult = await httpClient.action(
      api.fileStorageNodeActions.deleteById,
      {
        storageId,
      },
    );
    expect(deleteResult).toEqual(null);
    const getUrl = await httpClient.action(api.fileStorageNodeActions.getUrl, {
      storageId,
    });
    expect(getUrl).toEqual(null);
    const getResult = await httpClient.action(api.fileStorageNodeActions.get, {
      storageId: storageId,
    });
    expect(getResult).toEqual(null);
  });

  test("deletes with old id", async () => {
    const storageId = await httpClient.action(
      api.fileStorageNodeActions.store,
      {
        content: "helloworld",
        contentType: "text/plain",
      },
    );
    const metadata = await httpClient.action(
      api.fileStorageNodeActions.getMetadata,
      {
        storageId,
      },
    );
    const oldStorageId = (metadata as any).storageId;
    const deleteResult = await httpClient.action(
      api.fileStorageNodeActions.deleteById,
      {
        storageId: oldStorageId,
      },
    );
    expect(deleteResult).toEqual(null);
    const getUrl = await httpClient.action(api.fileStorageNodeActions.getUrl, {
      storageId: oldStorageId,
    });
    expect(getUrl).toEqual(null);
    const getResult = await httpClient.action(api.fileStorageNodeActions.get, {
      storageId: oldStorageId,
    });
    expect(getResult).toEqual(null);
  });

  test("getMetadata", async () => {
    const storageId = await httpClient.action(
      api.fileStorageNodeActions.store,
      {
        content: "helloworld",
        contentType: "text/plain",
      },
    );
    const metadata = await httpClient.action(
      api.fileStorageNodeActions.getMetadata,
      { storageId: storageId },
    );
    expect(metadata).toEqual(
      expect.objectContaining({
        sha256: sha256("helloworld"),
        size: "helloworld".length,
        contentType: "text/plain",
      }),
    );
  });

  test("getMetadata with old id", async () => {
    const storageId = await httpClient.action(
      api.fileStorageNodeActions.store,
      {
        content: "helloworld",
        contentType: "text/plain",
      },
    );
    const metadata = await httpClient.action(
      api.fileStorageNodeActions.getMetadata,
      { storageId: storageId },
    );
    expect(metadata).toEqual(
      expect.objectContaining({
        sha256: sha256("helloworld"),
        size: "helloworld".length,
        contentType: "text/plain",
      }),
    );

    const oldStorageId = (metadata as any).storageId;
    const metadata2 = await httpClient.action(
      api.fileStorageNodeActions.getMetadata,
      { storageId: oldStorageId },
    );
    expect(metadata2).toEqual({
      storageId: oldStorageId,
      sha256: sha256("helloworld"),
      size: "helloworld".length,
      contentType: "text/plain",
    });
  });

  test("error", async () => {
    await expect(
      httpClient.action(api.fileStorageNodeActions.getMetadata, {
        storageId: "INVALID",
      }),
    ).rejects.toThrow(/Invalid storage ID/);
  });
});

describe("File storage virtual tables", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("list and get", async () => {
    const postUrl = await httpClient.action(
      api.fileStorageNodeActions.generateUploadUrl,
      {},
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: {
        Digest: getDigestHeader("helloworld"),
        "Content-Type": "text/plain",
      },
    });

    const digest = sha256.array("helloworld");
    const base64Digest = btoa(String.fromCharCode(...digest));
    const hexDigest = sha256("helloworld");

    const listResult = await httpClient.query(api.fileStorage.list);
    expect(listResult.length).toEqual(1);
    expect(listResult[0]["contentType"]).toEqual("text/plain");
    const [major, minor] = version.split(".");
    if (+major > 1 || +minor >= 9) {
      expect(listResult[0]["sha256"]).toEqual(base64Digest);
    } else {
      expect(listResult[0]["sha256"]).toEqual(hexDigest);
    }
    expect(listResult[0]["size"]).toEqual("helloworld".length);

    const getResult = await httpClient.query(api.fileStorage.get, {
      id: listResult[0]["_id"],
    });
    expect(getResult).not.toEqual(null);
    expect(getResult!["contentType"]).toEqual("text/plain");
    if (+major > 1 || +minor >= 9) {
      expect(getResult!["sha256"]).toEqual(base64Digest);
    } else {
      expect(getResult!["sha256"]).toEqual(hexDigest);
    }
    expect(getResult!["size"]).toEqual("helloworld".length);
  });
});

describe("File storage in component", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("file storage upload and get url", async () => {
    const postUrl = await httpClient.mutation(
      api.fileStorageInComponent.generateUploadUrl,
      {},
    );
    expect(postUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/upload\\?token=.*`),
    );

    const response = await fetch(postUrl, {
      method: "POST",
      body: "helloworld",
      headers: { "Content-Type": "text/plain" },
    });
    const { storageId } = await response.json();

    // File exists in child component, not in parent.
    const listResult = await httpClient.query(api.fileStorageInComponent.list);
    expect(listResult.length).toEqual(1);
    const listInParentResult = await httpClient.query(api.fileStorage.list);
    expect(listInParentResult.length).toEqual(0);

    expect(listResult[0]["size"]).toEqual("helloworld".length);
    expect(listResult[0]._id).toEqual(storageId);

    const getResult = await httpClient.query(api.fileStorageInComponent.get, {
      id: storageId,
    });
    expect(getResult).not.toEqual(null);
    expect(getResult!["size"]).toEqual("helloworld".length);

    const getUrl = await httpClient.query(api.fileStorageInComponent.getUrl, {
      storageId,
    });
    expect(getUrl).toMatch(
      new RegExp(`${deploymentUrl}/api/storage/.*\\?component=.*`),
    );
    const getResponse = await fetch(getUrl, {
      method: "GET",
    });
    const text = await getResponse.text();
    expect(text).toEqual("helloworld");
  });

  test("file storage store and get", async () => {
    const storageId = await httpClient.action(
      api.fileStorageInComponent.storeFile,
      {
        data: "helloworld",
      },
    );

    // File exists in child component, not in parent.
    const listResult = await httpClient.query(api.fileStorageInComponent.list);
    expect(listResult.length).toEqual(1);
    const listInParentResult = await httpClient.query(api.fileStorage.list);
    expect(listInParentResult.length).toEqual(0);

    const text = await httpClient.action(api.fileStorageInComponent.getFile, {
      storageId,
    });
    expect(text).toEqual("helloworld");
  });
});
