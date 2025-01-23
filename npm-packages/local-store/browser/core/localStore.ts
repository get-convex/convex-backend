import { GenericDocument, SchemaDefinition } from "convex/server";
import { compareKeys } from "../../shared/compare";
import { ConvexSubscriptionId, IndexName, TableName } from "../../shared/types";
import { Page, Writes } from "./protocol";
import { cursorForSyncObject } from "../../shared/indexKeys";
import { GenericId } from "convex/values";

/**
 * documents:
 *  tableName
 *  id
 *  doc
 *
 *
 * indexPages:
 *   tableName
 *   indexName
 *   subscriptionId
 *   documents
 *
 */

export class CopyOnWriteLocalStore {
  private pagesByIndex: Map<
    TableName,
    Map<IndexName, Map<ConvexSubscriptionId, Page>>
  > = new Map();
  private writes: Writes = new Writes();
  private optimisticallyUpdatedPages: Map<ConvexSubscriptionId, Page> =
    new Map();

  private documents: Map<TableName, Map<GenericId<any>, GenericDocument>> =
    new Map();

  private optimisticallyUpdatedDocuments: Map<
    TableName,
    Map<GenericId<any>, GenericDocument | null>
  > = new Map();

  constructor(private schema: SchemaDefinition<any, any>) {}

  getChangedPages(other: CopyOnWriteLocalStore): Set<ConvexSubscriptionId> {
    // TODO -- actually diff
    const changedPages = new Set<ConvexSubscriptionId>();
    for (const [_tableName, indexPages] of this.pagesByIndex) {
      for (const [_indexName, subscriptionPages] of indexPages) {
        for (const [subscriptionId, _page] of subscriptionPages) {
          // if (
          //   other.pagesByIndex
          //     .get(tableName)
          //     ?.get(indexName)
          //     ?.get(subscriptionId) !== page
          // ) {
          changedPages.add(subscriptionId);
          // }
        }
      }
    }
    for (const [subscriptionId, _page] of other.optimisticallyUpdatedPages) {
      changedPages.add(subscriptionId);
    }
    // for (const [tableName, indexPages] of other.pagesByIndex) {
    //   for (const [indexName, subscriptionPages] of indexPages) {
    //     for (const [subscriptionId, page] of subscriptionPages) {
    //       if (
    //         this.pagesByIndex
    //           .get(tableName)
    //           ?.get(indexName)
    //           ?.get(subscriptionId) !== page
    //       ) {
    //         changedPages.add(subscriptionId);
    //       }
    //     }
    //   }
    // }
    return changedPages;
  }

  ingest(pages: Page[]) {
    for (const page of pages) {
      const indexPages = this.getPagesForIndex(page.tableName, page.indexName);
      indexPages.set(page.convexSubscriptionId, page);
      const tableDocuments = this.getTableDocuments(page.tableName);
      if (page.state.kind === "loaded") {
        for (const doc of page.state.value.results) {
          tableDocuments.set(doc._id as GenericId<any>, doc);
        }
      }
    }
  }

  loadObject(
    tableName: TableName,
    id: GenericId<any>,
  ): GenericDocument | null | undefined {
    const tableDocuments = this.getTableDocuments(tableName);
    const optimisticallyUpdatedTableDocuments =
      this.getOptimisticallyUpdatedTableDocuments(tableName);
    const optimisticallyUpdatedDocument =
      optimisticallyUpdatedTableDocuments.get(id);
    if (optimisticallyUpdatedDocument !== undefined) {
      return optimisticallyUpdatedDocument;
    }
    return tableDocuments.get(id);
  }

  private getTableDocuments(tableName: TableName) {
    if (!this.documents.has(tableName)) {
      this.documents.set(tableName, new Map());
    }
    return this.documents.get(tableName)!;
  }

  private getOptimisticallyUpdatedTableDocuments(tableName: TableName) {
    if (!this.optimisticallyUpdatedDocuments.has(tableName)) {
      this.optimisticallyUpdatedDocuments.set(tableName, new Map());
    }
    return this.optimisticallyUpdatedDocuments.get(tableName)!;
  }

  private getPagesForIndex(tableName: TableName, index: IndexName) {
    if (!this.pagesByIndex.has(tableName)) {
      this.pagesByIndex.set(tableName, new Map());
    }
    const indexPages = this.pagesByIndex.get(tableName)!;
    if (!indexPages.has(index)) {
      indexPages.set(index, new Map());
    }
    return indexPages.get(index)!;
  }

  private getPagesByIndex(tableName: TableName) {
    if (!this.pagesByIndex.has(tableName)) {
      this.pagesByIndex.set(tableName, new Map());
    }
    return this.pagesByIndex.get(tableName)!;
  }

  applyWrites(newWrites: Writes) {
    this.writes.apply(newWrites);
    for (const [tableName, tableWrites] of newWrites.writes.entries()) {
      for (const [id, document] of tableWrites.entries()) {
        const optimisticallyUpdatedDocuments =
          this.getOptimisticallyUpdatedTableDocuments(tableName);
        optimisticallyUpdatedDocuments.set(id, document);
        const tablePages = this.getPagesByIndex(tableName);
        for (const [index, subscriptionPages] of tablePages.entries()) {
          const indexKeyOrNull =
            document !== null
              ? cursorForSyncObject(this.schema, tableName, index, document)
              : null;
          for (const serverPage of subscriptionPages.values()) {
            const page = this.getPage(
              serverPage.tableName,
              serverPage.indexName,
              serverPage.convexSubscriptionId,
            );
            if (page === null) {
              throw new Error("page not found");
            }
            if (page.state.kind === "loading") {
              continue;
            }
            if (document === null) {
              throw new Error("Deletes not supported yet");
            }
            const isInRange =
              indexKeyOrNull !== null &&
              compareKeys(indexKeyOrNull, page.state.value.lowerBound) >= 0 &&
              compareKeys(indexKeyOrNull, page.state.value.upperBound) <= 0;

            const docs = page.state.value.results;
            const docIndex = docs.findIndex(
              (d: GenericDocument) => d.id === id,
            );
            // if the old document is in the page, remove it, and potentially
            // add the new document in the correct place
            if (docIndex !== -1) {
              const newDocuments = [
                ...docs.slice(0, docIndex),
                ...(isInRange ? [document] : []),
                ...docs.slice(docIndex + 1),
              ];
              newDocuments.sort((d1, d2) =>
                compareKeys(
                  cursorForSyncObject(this.schema, tableName, index, d1),
                  cursorForSyncObject(this.schema, tableName, index, d2),
                ),
              );
              const newPage = {
                ...page,
                state: {
                  kind: "loaded" as const,
                  value: {
                    ...page.state.value,
                    results: newDocuments,
                  },
                },
              };
              this.optimisticallyUpdatedPages.set(
                page.convexSubscriptionId,
                newPage,
              );
            }

            if (isInRange) {
              const newDocuments = [...page.state.value.results, document];
              newDocuments.sort((d1, d2) =>
                compareKeys(
                  cursorForSyncObject(this.schema, tableName, index, d1),
                  cursorForSyncObject(this.schema, tableName, index, d2),
                ),
              );
              const newPage = {
                ...page,
                state: {
                  kind: "loaded" as const,
                  value: {
                    ...page.state.value,
                    results: newDocuments,
                  },
                },
              };
              this.optimisticallyUpdatedPages.set(
                page.convexSubscriptionId,
                newPage,
              );
            }
          }
        }
      }
    }
  }

  debugInfo(tableName: TableName, index: IndexName) {
    const pagesForIndex = Array.from(
      this.pagesByIndex.get(tableName)?.get(index)?.values() ?? [],
    );
    return {
      pagesForIndex: pagesForIndex.map((page) =>
        page.state.kind === "loaded"
          ? {
              subscriptionId: page.convexSubscriptionId,
              results: page.state.value.results,
            }
          : {
              subscriptionId: page.convexSubscriptionId,
              target: page.state.target,
            },
      ),
      optimisticallyUpdatedPages: Array.from(
        this.optimisticallyUpdatedPages.entries(),
      ).map(([subscriptionId, page]) =>
        page.state.kind === "loaded"
          ? { subscriptionId, results: page.state.value.results }
          : { subscriptionId, target: page.state.target },
      ),
    };
  }

  getOrderedPages(tableName: TableName, index: IndexName) {
    // TODO: probably don't filter each time but whatever
    const pagesForIndex = Array.from(
      this.pagesByIndex.get(tableName)?.get(index)?.values() ?? [],
    );
    pagesForIndex.sort((p1, p2) =>
      compareKeys(
        p1.state.kind === "loaded"
          ? p1.state.value.lowerBound
          : p1.state.target,
        p2.state.kind === "loaded"
          ? p2.state.value.lowerBound
          : p2.state.target,
      ),
    );
    const pages: Page[] = [];
    for (const page of pagesForIndex) {
      if (this.optimisticallyUpdatedPages.has(page.convexSubscriptionId)) {
        pages.push(
          this.optimisticallyUpdatedPages.get(page.convexSubscriptionId)!,
        );
      } else {
        pages.push(page);
      }
    }
    return pages;
  }

  getPage(
    tableName: TableName,
    index: IndexName,
    convexSubscriptionId: ConvexSubscriptionId,
  ): Page | null {
    const page =
      this.pagesByIndex.get(tableName)?.get(index)?.get(convexSubscriptionId) ??
      null;
    if (page && this.optimisticallyUpdatedPages.has(convexSubscriptionId)) {
      return this.optimisticallyUpdatedPages.get(convexSubscriptionId)!;
    }
    return page;
  }

  clone(): CopyOnWriteLocalStore {
    const store = this.cloneWithoutWrites();
    store.optimisticallyUpdatedPages = new Map(
      Array.from(this.optimisticallyUpdatedPages.entries()),
    );
    store.writes = this.writes.clone();
    store.optimisticallyUpdatedDocuments = new Map(
      Array.from(this.optimisticallyUpdatedDocuments.entries()),
    );
    return store;
  }

  cloneWithoutWrites(): CopyOnWriteLocalStore {
    const store = new CopyOnWriteLocalStore(this.schema);
    store.pagesByIndex = new Map(
      Array.from(this.pagesByIndex.entries()).map(([tableName, indexPages]) => [
        tableName,
        new Map(
          Array.from(indexPages.entries()).map(([index, subscriptionPages]) => [
            index,
            new Map(subscriptionPages.entries()),
          ]),
        ),
      ]),
    );
    store.documents = new Map(
      Array.from(this.documents.entries()).map(
        ([tableName, tableDocuments]) => [
          tableName,
          new Map(Array.from(tableDocuments.entries())),
        ],
      ),
    );
    return store;
  }

  getAllPages(): Set<ConvexSubscriptionId> {
    return new Set(
      Array.from(this.pagesByIndex.values())
        .flatMap((indexPages) => Array.from(indexPages.values()))
        .flatMap((subscriptionPages) => Array.from(subscriptionPages.keys())),
    );
  }
}
