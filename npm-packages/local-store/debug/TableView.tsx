import { useEffect, useMemo, useState } from "react";
import { PageTimeline } from "./PageTimeline";
import { LocalStoreClient } from "../browser/ui";

export function TableViewer() {
  const syncQueryManager: LocalStoreClient = (globalThis as any).localDb;
  const [selectedTable, setSelectedTable] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<string | null>(null);

  // Parse schema to get tables and indexes
  const schema = JSON.parse((syncQueryManager.syncSchema as any).export());
  const tables = useMemo(() => schema.tables || [], [schema]);

  // Get indexes for selected table
  const selectedTableData = tables.find(
    (t: any) => t.tableName === selectedTable,
  );
  const indexes = selectedTableData?.indexes || [];

  // Set default selections on mount
  useEffect(() => {
    if (tables.length > 0 && !selectedTable) {
      const firstTable = tables[0].tableName;
      setSelectedTable(firstTable);

      // Also set the first index for this table
      const firstTableIndexes = tables[0].indexes || [];
      if (firstTableIndexes.length > 0) {
        setSelectedIndex(firstTableIndexes[0].indexDescriptor);
      }
    }
  }, [tables, selectedTable]);

  // Reset index selection when table changes
  //   useEffect(() => {
  //     if (selectedTable && indexes.length > 0) {
  //       setSelectedIndex(indexes[0].indexDescriptor);
  //     } else {
  //       setSelectedIndex(null);
  //     }
  //   }, [selectedTable, indexes]);

  // Get pages for selected table/index
  // NOTE this doesn't include optimistic updates
  // TODO fix this
  const pages: any[] = [];
  // selectedTable && selectedIndex
  //   ? syncQueryManager.coreLocalStoregetOrderedPages(
  //       selectedTable,
  //       selectedIndex,
  //     )
  //   : [];

  return (
    <div className="space-y-4">
      <div className="flex gap-4">
        {/* Table Selector */}
        <div className="flex-1">
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Table
          </label>
          <select
            value={selectedTable || ""}
            onChange={(e) => setSelectedTable(e.target.value || null)}
            className="block w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
          >
            <option value="">Select a table...</option>
            {tables.map((table: any) => (
              <option key={table.tableName} value={table.tableName}>
                {table.tableName}
              </option>
            ))}
          </select>
        </div>

        {/* Index Selector */}
        <div className="flex-1">
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Index
          </label>
          <select
            value={selectedIndex || ""}
            onChange={(e) => setSelectedIndex(e.target.value || null)}
            className="block w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            disabled={!selectedTable}
          >
            <option value="">Select an index...</option>
            {indexes.map((index: any) => (
              <option key={index.indexDescriptor} value={index.indexDescriptor}>
                {index.indexDescriptor} ({index.fields.join(", ")})
              </option>
            ))}
          </select>
        </div>
      </div>

      {selectedTable && selectedIndex && (
        <div className="mt-8 space-y-2 h-96">
          <div>Pages ({pages.length})</div>
          {/* Add the new timeline visualization */}
          {selectedTable && selectedIndex && pages.length > 0 && (
            <PageTimeline
              syncSchema={syncQueryManager.syncSchema}
              orderedPages={pages}
            />
          )}
        </div>
      )}
    </div>
  );
}
