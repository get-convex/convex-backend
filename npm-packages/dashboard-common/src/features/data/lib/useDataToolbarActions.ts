import { Cursor, GenericDocument } from "convex/server";
import { ConvexError } from "convex/values";
import { captureException } from "@sentry/nextjs";
import { useMutation } from "convex/react";
import udfs from "udfs";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { useInvalidateShapes, useDeleteTables } from "features/data/lib/api";
import { useNents } from "lib/useNents";
import { toast } from "lib/utils";

export function useDataToolbarActions({
  tableName,
  handleAddDocuments,
  clearSelectedRows,
  loadMore,
}: {
  tableName: string;
  handleAddDocuments(): void;
  clearSelectedRows(): void;
  loadMore(): void;
}): {
  addDocuments: (documents: GenericDocument[]) => Promise<void>;
  patchFields: (
    rowIds: Set<string> | "all",
    fields: GenericDocument,
  ) => Promise<void>;
  deleteTable: () => Promise<void>;
  clearTable: (
    cursor: Cursor | null,
  ) => Promise<{ continueCursor: Cursor; deleted: number; hasMore: boolean }>;
  deleteRows: (rowIds: Set<string>) => Promise<void>;
} {
  const invalidateShapes = useInvalidateShapes();

  const documentAdd = useMutation(udfs.addDocument.default);
  const { selectedNent } = useNents();
  const addDocuments = async (documents: GenericDocument[]) => {
    try {
      await documentAdd({
        table: tableName,
        documents,
        componentId: selectedNent?.id ?? null,
      });
    } catch (error: any) {
      if (error instanceof ConvexError) {
        throw new Error(error.data);
      }
      throw error;
    }
    handleAddDocuments();
    await invalidateShapes();
  };

  const deleteDocuments = useMutation(udfs.deleteDocuments.default);

  const tableClear = useMutation(udfs.clearTablePage.default);
  const patchDocumentsFields = useMutation(udfs.patchDocumentsFields.default);

  const patchFields = async (
    rowIds: Set<string> | "all",
    fields: GenericDocument,
  ) => {
    try {
      await patchDocumentsFields({
        table: tableName as any,
        fields,

        ids: rowIds === "all" ? undefined : (Array.from(rowIds) as Id<any>[]),
        componentId: selectedNent?.id ?? null,
      });
    } catch (error: any) {
      if (error instanceof ConvexError) {
        throw new Error(error.data);
      }
      throw new Error(
        `Could not bulk edit documents. Try editing less documents or use a migration.\n\n${error.toString()}`,
      );
    }
  };

  const clearTable = async (
    cursor: Cursor | null,
  ): Promise<{ continueCursor: Cursor; deleted: number; hasMore: boolean }> =>
    tableClear({ tableName, cursor, componentId: selectedNent?.id ?? null });

  const tableDelete = useDeleteTables();
  const deleteTable = async () => {
    try {
      const resp = await tableDelete([tableName], selectedNent?.id ?? null);
      if (!resp?.success) {
        toast("error", resp.error);
      } else {
        toast("success", "Table deleted.");
        await invalidateShapes();
      }
    } catch (err: any) {
      captureException(err);
      toast(
        "error",
        "An unknown error occurred. Please try refreshing the page.",
      );
    }
  };
  const deleteRows = async (rowIds: Set<string>) => {
    if (rowIds.size === 0) {
      return;
    }
    try {
      const resp = await deleteDocuments({
        toDelete: Array.from(rowIds).map((id) => ({ tableName, id })),
        componentId: selectedNent?.id ?? null,
      });
      if (!resp?.success) {
        toast("error", resp.error);
      } else {
        toast(
          "success",
          `Deleted ${rowIds.size} ${
            rowIds.size === 1 ? "document" : "documents"
          }.`,
        );
        await invalidateShapes();
        clearSelectedRows();
        loadMore();
      }
    } catch (err: any) {
      captureException(err);
      toast(
        "error",
        "An unknown error occurred. Please try refreshing the page.",
      );
    }
  };

  return {
    addDocuments,
    patchFields,
    deleteTable,
    clearTable,
    deleteRows,
  };
}
