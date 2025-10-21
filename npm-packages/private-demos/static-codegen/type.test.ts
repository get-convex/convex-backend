/**
 * Type test: Compare DataModel generated from schema definition vs. static codegen
 *
 * This file tests that the statically generated DataModel type from codegen
 * exactly matches the DataModel type inferred from the schema definition.
 *
 * Run `npx convex dev` in this directory to generate the types, then run
 * this test with vitest to verify the types match.
 */

import { test, expectTypeOf, expect } from "vitest";
import { DataModelFromSchemaDefinition } from "convex/server";
import { DataModel } from "./convex/_generated/dataModel";
import schema from "./convex/schema";

type SchemaDataModel = DataModelFromSchemaDefinition<typeof schema>;

test("static codegen DataModel matches schema-inferred DataModel", () => {
  expect(schema.tables.empty).toBeDefined();
  // The main type test: schema-inferred DataModel should exactly match generated DataModel
  expectTypeOf<SchemaDataModel>().toEqualTypeOf<DataModel>();

  // Additional tests for specific tables to provide better error messages

  // Test: empty table
  expectTypeOf<SchemaDataModel["empty"]>().toEqualTypeOf<DataModel["empty"]>();

  // Test: primitiveTypes table (with all primitive types, indexes, search, and vector)
  expectTypeOf<SchemaDataModel["primitiveTypes"]>().toEqualTypeOf<
    DataModel["primitiveTypes"]
  >();
  expectTypeOf<SchemaDataModel["primitiveTypes"]["document"]>().toEqualTypeOf<
    DataModel["primitiveTypes"]["document"]
  >();
  expectTypeOf<SchemaDataModel["primitiveTypes"]["indexes"]>().toEqualTypeOf<
    DataModel["primitiveTypes"]["indexes"]
  >();
  expectTypeOf<
    SchemaDataModel["primitiveTypes"]["searchIndexes"]
  >().toEqualTypeOf<DataModel["primitiveTypes"]["searchIndexes"]>();
  expectTypeOf<
    SchemaDataModel["primitiveTypes"]["vectorIndexes"]
  >().toEqualTypeOf<DataModel["primitiveTypes"]["vectorIndexes"]>();

  // Test: objectTypes table with nested fields and indexes on nested paths
  expectTypeOf<SchemaDataModel["objectTypes"]>().toEqualTypeOf<
    DataModel["objectTypes"]
  >();
  expectTypeOf<SchemaDataModel["objectTypes"]["fieldPaths"]>().toEqualTypeOf<
    DataModel["objectTypes"]["fieldPaths"]
  >();
  expectTypeOf<SchemaDataModel["objectTypes"]["indexes"]>().toEqualTypeOf<
    DataModel["objectTypes"]["indexes"]
  >();

  // Test: topLevelUnion table with union at document level
  expectTypeOf<SchemaDataModel["topLevelUnion"]>().toEqualTypeOf<
    DataModel["topLevelUnion"]
  >();
  expectTypeOf<SchemaDataModel["topLevelUnion"]["document"]>().toEqualTypeOf<
    DataModel["topLevelUnion"]["document"]
  >();
  expectTypeOf<SchemaDataModel["topLevelUnion"]["indexes"]>().toEqualTypeOf<
    DataModel["topLevelUnion"]["indexes"]
  >();

  // Test: unionTypes table with unions, literals, and optional unions
  expectTypeOf<SchemaDataModel["unionTypes"]>().toEqualTypeOf<
    DataModel["unionTypes"]
  >();
  expectTypeOf<SchemaDataModel["unionTypes"]["document"]>().toEqualTypeOf<
    DataModel["unionTypes"]["document"]
  >();
  expectTypeOf<SchemaDataModel["unionTypes"]["indexes"]>().toEqualTypeOf<
    DataModel["unionTypes"]["indexes"]
  >();
  expectTypeOf<SchemaDataModel["unionTypes"]["searchIndexes"]>().toEqualTypeOf<
    DataModel["unionTypes"]["searchIndexes"]
  >();
  expectTypeOf<SchemaDataModel["unionTypes"]["vectorIndexes"]>().toEqualTypeOf<
    DataModel["unionTypes"]["vectorIndexes"]
  >();

  // Test: recordTypes table with different key types and indexes
  expectTypeOf<SchemaDataModel["recordTypes"]>().toEqualTypeOf<
    DataModel["recordTypes"]
  >();
  expectTypeOf<SchemaDataModel["recordTypes"]["document"]>().toEqualTypeOf<
    DataModel["recordTypes"]["document"]
  >();
  expectTypeOf<SchemaDataModel["recordTypes"]["fieldPaths"]>().toEqualTypeOf<
    DataModel["recordTypes"]["fieldPaths"]
  >();
  expectTypeOf<SchemaDataModel["recordTypes"]["searchIndexes"]>().toEqualTypeOf<
    DataModel["recordTypes"]["searchIndexes"]
  >();
});
