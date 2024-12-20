import { GenericDocument } from "convex/server";
import { IndexRangeRequest } from "../shared/types";

import { IndexRangeBounds, TableName } from "../shared/types";
import { GenericId } from "convex/values";
import { LocalDbReaderImpl } from "./localDbReader";

export class LocalDbWriterImpl extends LocalDbReaderImpl {
  public debugIndexRanges: Map<
    string,
    {
      table: string;
      index: string;
      indexRangeBounds: IndexRangeBounds;
      order: "asc" | "desc";
      limit: number;
    }
  > = new Map();
  private recordWrite: (
    tableName: TableName,
    id: GenericId<any>,
    doc: GenericDocument | null,
  ) => void;
  constructor(
    syncSchema: any,
    requestRange: (rangeRequest: IndexRangeRequest) => Array<GenericDocument>,
    loadObject: (
      table: TableName,
      id: GenericId<any>,
    ) => GenericDocument | null,
    recordWrite: (
      tableName: TableName,
      id: GenericId<any>,
      doc: GenericDocument | null,
    ) => void,
  ) {
    super(syncSchema, requestRange, loadObject);
    this.recordWrite = recordWrite;
  }

  insert(
    tableName: TableName,
    id: GenericId<any>,
    doc: GenericDocument | null,
  ) {
    this.recordWrite(tableName, id, {
      ...doc,
      _creationTime: Date.now(),
      _id: id,
    });
  }

  delete(tableName: TableName, id: GenericId<any>) {
    this.recordWrite(tableName, id, null);
  }

  replace(tableName: TableName, id: GenericId<any>, doc: GenericDocument) {
    this.recordWrite(tableName, id, doc);
  }

  patch(tableName: TableName, id: GenericId<any>, update: GenericDocument) {
    const existing = this.get(tableName, id);
    if (existing === null) {
      throw new Error("Object not found");
    }
    this.recordWrite(tableName, id, {
      ...existing,
      ...update,
    });
  }
}
