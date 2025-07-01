#!/usr/bin/env node

import { readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Paths to keep in the filtered API
const PATHS_TO_KEEP = [
  "/teams",
  "/teams/{team_id}/projects",
  "/projects/{project_id}/instances",
  "/instances/{deployment_name}/auth",
  "/create_project",
  "/projects/{project_id}/transfer",
  "/delete_project/{project_id}",
];

function collectReferencedSchemas(obj, collected = new Set()) {
  if (typeof obj !== "object" || obj === null) {
    return collected;
  }

  if (Array.isArray(obj)) {
    obj.forEach((item) => collectReferencedSchemas(item, collected));
    return collected;
  }

  for (const [key, value] of Object.entries(obj)) {
    if (
      key === "$ref" &&
      typeof value === "string" &&
      value.startsWith("#/components/schemas/")
    ) {
      const schemaName = value.replace("#/components/schemas/", "");
      collected.add(schemaName);
    } else {
      collectReferencedSchemas(value, collected);
    }
  }

  return collected;
}

function filterOpenApiSpec(inputPath, outputPath) {
  console.log(`Reading OpenAPI spec from: ${inputPath}`);

  const openApiSpec = JSON.parse(readFileSync(inputPath, "utf8"));

  // Create filtered spec with the same structure
  const filteredSpec = {
    openapi: openApiSpec.openapi,
    info: openApiSpec.info,
    paths: {},
    components: {
      schemas: {},
    },
  };

  // Filter paths
  console.log("Filtering paths...");
  for (const path of PATHS_TO_KEEP) {
    if (openApiSpec.paths[path]) {
      filteredSpec.paths[path] = openApiSpec.paths[path];
      console.log(`✓ Kept path: ${path}`);
    } else {
      console.warn(`⚠ Path not found: ${path}`);
    }
  }

  // Collect all referenced schemas
  console.log("Collecting referenced schemas...");
  const referencedSchemas = collectReferencedSchemas(filteredSpec.paths);

  // Recursively collect schemas that reference other schemas
  let schemasToProcess = [...referencedSchemas];
  while (schemasToProcess.length > 0) {
    const schemaName = schemasToProcess.pop();
    if (
      !filteredSpec.components.schemas[schemaName] &&
      openApiSpec.components?.schemas?.[schemaName]
    ) {
      const schema = openApiSpec.components.schemas[schemaName];
      filteredSpec.components.schemas[schemaName] = schema;

      // Find any additional references in this schema
      const additionalRefs = collectReferencedSchemas(schema);
      for (const ref of additionalRefs) {
        if (
          !referencedSchemas.has(ref) &&
          !filteredSpec.components.schemas[ref]
        ) {
          referencedSchemas.add(ref);
          schemasToProcess.push(ref);
        }
      }
    }
  }

  console.log(
    `Kept ${Object.keys(filteredSpec.paths).length} paths and ${Object.keys(filteredSpec.components.schemas).length} schemas`,
  );
  console.log("Referenced schemas:", [...referencedSchemas].sort());

  // Write filtered spec
  console.log(`Writing filtered spec to: ${outputPath}`);
  writeFileSync(outputPath, JSON.stringify(filteredSpec, null, 2));
  console.log("✓ Filtering complete");
}

// Main execution
const inputPath = join(__dirname, "../../dashboard/dashboard-openapi.json");
const outputPath = join(__dirname, "./src/filtered-openapi.json");

try {
  filterOpenApiSpec(inputPath, outputPath);
} catch (error) {
  console.error("Error filtering OpenAPI spec:", error);
  process.exit(1);
}
