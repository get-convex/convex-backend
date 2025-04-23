import { describe, test, expect, beforeEach, vi, expectTypeOf } from "vitest";
import { convexTest } from "convex-test";
import { api } from "../../_generated/api";
import { modules } from "../../../setup.test";

vi.mock("../server");

describe("fileStorageV2", () => {
  let t: ReturnType<typeof convexTest>;

  beforeEach(() => {
    t = convexTest(undefined, modules);
  });

  describe("fileMetadata", () => {
    test("returns empty result when no files exist", async () => {
      const result = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
        },
      );

      expect(result.page).toHaveLength(0);
      expect(result.isDone).toBe(true);
    });

    test("returns files with URLs in desc order by default", async () => {
      await t.run(async (ctx) => {
        await ctx.storage.store(new Blob(["test content 1"]));
        await ctx.storage.store(new Blob(["test content 2"]));
      });

      const result = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
        },
      );

      expect(result.page).toHaveLength(2);
      expect(result.isDone).toBe(true);

      // Files should be in desc order by default
      expect(result.page[0]._creationTime).toBeGreaterThan(
        result.page[1]._creationTime,
      );

      // Each file should have a URL
      expect(result.page[0].url).toBeDefined();
      expect(result.page[1].url).toBeDefined();
    });

    test("respects filters for date range", async () => {
      await t.run(async (ctx) => {
        // Create files with different creation times
        await ctx.storage.store(new Blob(["test content 1"]));
        await ctx.storage.store(new Blob(["test content 2"]));
        await ctx.storage.store(new Blob(["test content 3"]));
      });

      // Get the second item's creation time to use as filter
      const allFiles = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
        },
      );
      const secondItemTime = allFiles.page[1]._creationTime;

      // Test minCreationTime filter using second item's time
      const resultMinCreationTime = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
          filters: { minCreationTime: secondItemTime },
        },
      );
      expect(resultMinCreationTime.page).toHaveLength(2);
      expect(
        resultMinCreationTime.page[0]._creationTime,
      ).toBeGreaterThanOrEqual(secondItemTime);
      expect(
        resultMinCreationTime.page[1]._creationTime,
      ).toBeGreaterThanOrEqual(secondItemTime);

      // Test maxCreationTime filter
      const resultMaxCreationTime = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
          filters: { maxCreationTime: secondItemTime },
        },
      );
      expect(resultMaxCreationTime.page).toHaveLength(2);
      expect(resultMaxCreationTime.page[0]._creationTime).toBeLessThanOrEqual(
        secondItemTime,
      );
      expect(resultMaxCreationTime.page[1]._creationTime).toBeLessThanOrEqual(
        secondItemTime,
      );

      // Test both minCreationTime and maxCreationTime
      const resultRange = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
          filters: {
            minCreationTime: secondItemTime,
            maxCreationTime: secondItemTime,
          },
        },
      );
      expect(resultRange.page).toHaveLength(1);
      expect(resultRange.page[0]._creationTime).toBe(secondItemTime);
    });

    test("respects order parameter", async () => {
      await t.run(async (ctx) => {
        await ctx.storage.store(new Blob(["test content 1"]));
        await ctx.storage.store(new Blob(["test content 2"]));
      });

      // Test ascending order
      const resultAsc = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
          filters: { order: "asc" },
        },
      );
      expect(resultAsc.page).toHaveLength(2);
      expectTypeOf(resultAsc.page[0]._creationTime).toBeNumber();
      expectTypeOf(resultAsc.page[1]._creationTime).toBeNumber();
      expect(resultAsc.page[0]._creationTime).toBeLessThan(
        resultAsc.page[1]._creationTime,
      );

      // Test descending order
      const resultDesc = await t.query(
        api._system.frontend.fileStorageV2.fileMetadata,
        {
          paginationOpts: { numItems: 10, cursor: null },
          filters: { order: "desc" },
        },
      );
      expect(resultDesc.page).toHaveLength(2);
      expectTypeOf(resultDesc.page[0]._creationTime).toBeNumber();
      expectTypeOf(resultDesc.page[1]._creationTime).toBeNumber();
      expect(resultDesc.page[0]._creationTime).toBeGreaterThan(
        resultDesc.page[1]._creationTime,
      );
    });
  });

  describe("getFile", () => {
    test("returns null for non-existent file", async () => {
      // Create a file first
      const fileId = await t.run(async (ctx) => {
        return await ctx.storage.store(new Blob(["test content"]));
      });

      // Delete the file
      await t.mutation(api._system.frontend.fileStorageV2.deleteFile, {
        storageId: fileId,
      });

      // Verify file is now null when queried
      const result = await t.query(api._system.frontend.fileStorageV2.getFile, {
        storageId: fileId,
      });
      expect(result).toBeNull();
    });
  });

  describe("deleteFile", () => {
    test("deletes existing file", async () => {
      const fileId = await t.run(async (ctx) => {
        return await ctx.storage.store(new Blob(["test content"]));
      });

      await t.mutation(api._system.frontend.fileStorageV2.deleteFile, {
        storageId: fileId,
      });

      // Verify file was deleted
      const checkFile = await t.run(async (ctx) => {
        return await ctx.db.system.get(fileId);
      });
      expect(checkFile).toBeNull();
    });
  });

  describe("deleteFiles", () => {
    test("deletes multiple files", async () => {
      const fileIds = await t.run(async (ctx) => {
        const id1 = await ctx.storage.store(new Blob(["test content 1"]));
        const id2 = await ctx.storage.store(new Blob(["test content 2"]));
        return [id1, id2];
      });

      await t.mutation(api._system.frontend.fileStorageV2.deleteFiles, {
        storageIds: fileIds,
      });

      // Verify files were deleted
      const checkFiles = await t.run(async (ctx) => {
        const file1 = await ctx.storage.get(fileIds[0]);
        const file2 = await ctx.storage.get(fileIds[1]);
        return { file1, file2 };
      });

      expect(checkFiles.file1).toBeNull();
      expect(checkFiles.file2).toBeNull();
    });
  });

  describe("generateUploadUrl", () => {
    test("returns a URL string", async () => {
      const url = await t.mutation(
        api._system.frontend.fileStorageV2.generateUploadUrl,
        {},
      );
      expect(typeof url).toBe("string");
      expect(url).toBeTruthy();
    });
  });
});
