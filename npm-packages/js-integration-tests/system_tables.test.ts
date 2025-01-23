import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

describe("ConvexHttpClient", () => {
  let client: ConvexHttpClient;

  beforeEach(async () => {
    client = new ConvexHttpClient(deploymentUrl);
    await client.mutation(api.systemTables.scheduleJob);
    await client.mutation(api.messages.sendMessage, {
      channel: "A",
      text: "A",
    });
  });

  afterEach(async () => {
    await client.mutation(api.cleanUp.default);
  });

  test("All system tables can be accessed via db.system", async () => {
    const exampleUdf = async () => {
      await client.query(api.systemTables.queryAll);
    };
    await expect(exampleUdf()).resolves.not.toThrow();
  });

  // query functionality

  test("db.system.query() succeeds on system tables", async () => {
    const exampleUdf = async () => {
      const results = await client.query(api.systemTables.listJobs);
      expect(results.length).toBeGreaterThan(0);
    };
    await expect(exampleUdf()).resolves.not.toThrow();
  });

  test("db.system.query() fails with user tables", async () => {
    await expect(
      client.query(api.systemTables.badSystemQuery),
    ).rejects.toThrow();
  });

  test("db.query() fails with system tables", async () => {
    await expect(client.query(api.systemTables.badUserQuery)).rejects.toThrow();
  });

  // get functionality
  test("db.system.get() on system-table id", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      const result = await client.query(api.systemTables.getJob, { id: jobId });
      expect(result).not.toBeNull();
      expect(result!._id).toEqual(jobId);
    };
    await expect(exampleUdf()).resolves.not.toThrow();
  });

  test("db.system.get() fails with user-table id", async () => {
    const exampleUdf = async () => {
      const messages = await client.query(api.systemTables.listMessages);
      expect(messages.length).toBeGreaterThan(0);
      const messageId = messages[0]._id;
      await client.query(api.systemTables.badSystemGet, {
        id: messageId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("db.get() fails with system-table id", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.query(api.systemTables.badUserGet, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  // assert that system tables are read only
  test("db.insert() fails with user tables", async () => {
    await expect(
      client.mutation(api.systemTables.badSystemInsert),
    ).rejects.toThrow();
  });

  test("db.patch() fails with system-table id", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.mutation(api.systemTables.badSystemPatch, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("db.replace() fails with system-table id", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.mutation(api.systemTables.badSystemReplace, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("db.delete() fails with system-table id", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.mutation(api.systemTables.badSystemDelete, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  // assert that db.system object doesn't have any writer methods
  test("db.system doesn't have insert()", async () => {
    const exampleUdf = async () => {
      await client.mutation(api.systemTables.systemInsertJSError);
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("db.system doesn't have patch()", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.mutation(api.systemTables.systemPatchJSError, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("db.system doesn't have replace()", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.mutation(api.systemTables.systemReplaceJSError, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("db.system doesn't have delete()", async () => {
    const exampleUdf = async () => {
      const jobs = await client.query(api.systemTables.listJobs);
      expect(jobs.length).toBeGreaterThan(0);
      const jobId = jobs[0]._id;
      await client.mutation(api.systemTables.systemDeleteJSError, {
        id: jobId,
      });
    };
    await expect(exampleUdf()).rejects.toThrow();
  });

  test("virtual ids can be in schema validation", async () => {
    const exampleUdf = async () => {
      const doc_id = await client.mutation(
        api.systemTables.setForeignVirtualId,
      );
      expect(doc_id).not.toBeNull();
    };
    await expect(exampleUdf()).resolves.not.toThrow();
  });
});
