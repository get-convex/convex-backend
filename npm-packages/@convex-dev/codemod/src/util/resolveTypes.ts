import { Project, Type } from "ts-morph";
import { join } from "path";

const convexExports = ["anyDatabaseReader", "anyDatabaseWriter", "id"] as const;

/**
 * This exports a few TypeScript types that are used in the codemod.
 *
 * To do so, we create a temporary file that references the relevant types,
 * and we add it to the ts-morph project. This temporary file must be deleted
 * after the types are used (see {@link deleteResolveTypesTempFile}).
 */
export function resolveTypes(
  project: Project,
  root: string,
): Record<(typeof convexExports)[number], Type> {
  const tempFileName = getTempFilePath(root);

  const source = `
    import { GenericDatabaseReader, GenericDatabaseWriter } from "convex/server";
    import { Id } from "convex/values";

    declare const anyDatabaseReader: GenericDatabaseReader<any>;
    declare const anyDatabaseWriter: GenericDatabaseWriter<any>;
    declare const id: Id<any>;
  `;
  const tempFile = project.createSourceFile(tempFileName, source);

  const result = Object.fromEntries(
    convexExports.map((name) => {
      const exportDeclaration = tempFile.getVariableDeclarationOrThrow(name);
      return [name, exportDeclaration.getType()];
    }),
  );

  return result as any;
}

export function deleteResolveTypesTempFile(project: Project, root: string) {
  const tempFileName = getTempFilePath(root);
  const tempFile = project.getSourceFileOrThrow(tempFileName);
  tempFile.delete();
}

function getTempFilePath(root: string) {
  return join(root, `_convex_codemod_importResolution.ts`);
}
