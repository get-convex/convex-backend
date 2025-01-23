import { GenericDocument, IndexRange } from "convex/server";
import { DocumentByInfo, NamedIndex } from "convex/server";
import { GenericTableInfo } from "convex/server";
import { IndexRangeBuilder } from "convex/server";
import { IndexName, IndexPrefix, IndexRangeRequest } from "../shared/types";

import { IndexRangeBounds, TableName } from "../shared/types";
import { PaginatorIndexRange } from "../shared/pagination";
import { GenericId } from "convex/values";
import { indexFieldsForSyncObject } from "../server/resolvers";

export class LocalDbReaderImpl {
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
  constructor(
    private syncSchema: any,
    private requestRange: (
      rangeRequest: IndexRangeRequest,
    ) => Array<GenericDocument>,
    private loadObject: (
      table: TableName,
      id: GenericId<any>,
    ) => GenericDocument | null,
  ) {}

  query(table: TableName) {
    return {
      fullTableScan: () => {
        throw new Error("Not implemented");
      },
      withIndex: (
        indexName: IndexName,
        indexBuilder?: (
          q: IndexRangeBuilder<
            DocumentByInfo<GenericTableInfo>,
            NamedIndex<GenericTableInfo, string>
          >,
        ) => IndexRange,
      ) => {
        return {
          collect: () => this.queryRange(table, indexName, indexBuilder, "asc"),
          take: (count: number) =>
            this.queryRange(table, indexName, indexBuilder, "asc", count),
          order: (order: "asc" | "desc") => {
            return {
              collect: () =>
                this.queryRange(
                  table,
                  indexName,
                  indexBuilder,
                  order,
                  Number.POSITIVE_INFINITY,
                ),
              take: (count: number) =>
                this.queryRange(table, indexName, indexBuilder, order, count),
            };
          },
        };
      },
      withSearchIndex: (_indexName: any, _indexBuilder: any) => {
        throw new Error("Not implemented");
      },
    };
  }

  get(table: TableName, id: GenericId<any>) {
    return this.loadObject(table, id);
  }

  private queryRange(
    table: TableName,
    index: IndexName,
    indexRange?: (
      q: IndexRangeBuilder<
        DocumentByInfo<GenericTableInfo>,
        NamedIndex<GenericTableInfo, string>
      >,
    ) => IndexRange,
    order: "asc" | "desc" = "asc",
    count: number = Number.POSITIVE_INFINITY,
  ): GenericDocument[] {
    console.log("queryRange", table, index, order, count);
    // TODO: should do something like this
    // or even better, use the DatabaseImpl wholesale from convex/server
    // const indexRangeJson = (indexRange(IndexRangeBuilderImpl.new()) as any).export()
    // But that's not exported from convex/server yet.
    // So we do the hack using the version in convex-helpers.
    const indexFields = indexFieldsForSyncObject(this.syncSchema, table, index);
    const paginatorIndexRange = new PaginatorIndexRange(indexFields);
    if (indexRange) {
      indexRange(paginatorIndexRange as any);
    }
    const indexRangeBounds: IndexRangeBounds = {
      lowerBound:
        paginatorIndexRange.lowerBoundIndexKey ??
        ([] as unknown as IndexPrefix),
      lowerBoundInclusive: paginatorIndexRange.lowerBoundInclusive,
      upperBound:
        paginatorIndexRange.upperBoundIndexKey ??
        ([] as unknown as IndexPrefix),
      upperBoundInclusive: paginatorIndexRange.upperBoundInclusive,
    };
    // requestRange will cancel the query execution if the result is not loaded,
    // and make the sync query return loading
    return this.requestRange({
      tableName: table,
      indexName: index,
      count,
      indexRangeBounds,
      order,
    });
  }
}

export class LoadingError extends Error {
  constructor() {
    super("Index range is loading in sync query");
  }
}
