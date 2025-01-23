export function indexFieldsForSyncObject(
  syncSchema: any,
  table: string,
  index: string,
) {
  const indexDefinition: any = syncSchema.tables[table].indexes.find(
    (i: any) => i.indexDescriptor === index,
  );
  if (!indexDefinition) {
    throw new Error(`Index ${index} not found for table ${table}`);
  }
  return indexDefinition.fields;
}

export function cursorForSyncObject(
  syncSchema: any,
  table: string,
  index: string,
  doc: any,
) {
  const fields = indexFieldsForSyncObject(syncSchema, table, index);
  // TODO: null is kind of wrong but we can't use undefined because it's not convex-json serializable
  return {
    kind: "exact" as const,
    value: fields.map((field: string) => doc[field] ?? null),
  };
}
