import { Context, startLogProgress } from "@/context";
import { deleteResolveTypesTempFile, resolveTypes } from "@/util/resolveTypes";
import path from "node:path";
import {
  CallExpression,
  Project,
  SourceFile,
  SyntaxKind,
  ts,
  Type,
} from "ts-morph";

export const CONVEX_VERSION_RANGE = ">=1.99.0";

export async function explicitIds(
  ctx: Context,
  project: Project,
  root: string,
) {
  const progress = startLogProgress(
    ctx,
    "[:bar] :percent (:current/:total) Processing :name…",
    {
      width: 30,
      total: project.getSourceFiles().length,
    },
  );

  for (const sourceFile of project.getSourceFiles()) {
    progress.tick({
      name: path.relative(root, sourceFile.getFilePath()),
    });

    // Note: we create and delete the temporary file for every file in the
    // source tree. This is intentional, because it looks like if we only
    // create it once, the resolved types break when processing the next file
    // after modifying the first one.
    const { anyDatabaseReader, anyDatabaseWriter, id } = resolveTypes(
      project,
      root,
    );

    const dbCalls = findDbCalls(
      sourceFile,
      anyDatabaseReader,
      anyDatabaseWriter,
    );

    for (const call of dbCalls) {
      updateDbCall(ctx, call, id);
    }

    deleteResolveTypesTempFile(project, root);
    await allowInterrupt();
  }

  progress.terminate();
}

function findDbCalls(
  sourceFile: SourceFile,
  anyDatabaseReader: Type,
  anyDatabaseWriter: Type,
): CallExpression<ts.CallExpression>[] {
  return sourceFile
    .getDescendantsOfKind(SyntaxKind.CallExpression)
    .filter((node) => {
      const callee = node.getExpression();

      // Check if the callee is a property access expression (XXX.get)
      const propertyAccess = callee.asKind(SyntaxKind.PropertyAccessExpression);
      if (!propertyAccess) {
        return false;
      }

      const methodName = propertyAccess.getNameNode().getText();
      if (!["get", "replace", "patch", "delete"].includes(methodName)) {
        return false;
      }

      const dbType = propertyAccess.getExpression().getType();

      const isDbCall =
        (dbType.isAssignableTo(anyDatabaseReader) && methodName === "get") ||
        dbType.isAssignableTo(anyDatabaseWriter);
      if (!isDbCall) {
        return false;
      }

      // Is it an unmigrated call?
      const args = node.getArguments();
      return (
        (methodName === "get" && args.length === 1) ||
        (methodName === "replace" && args.length === 2) ||
        (methodName === "patch" && args.length === 2) ||
        (methodName === "delete" && args.length === 1)
      );
    });
}

function updateDbCall(
  ctx: Context,
  call: CallExpression<ts.CallExpression>,
  id: Type,
) {
  const idArg = call.getArguments()[0]!;
  const idType = idArg.getType();

  if (!idType.isAssignableTo(id)) {
    ctx.addWarning({
      title: "Can’t update call site",
      message: `Expected \`${idArg.getText()}\` to be an \`Id<T>\`, but it is an \`${idType.getText()}\` instead.`,
      node: idArg,
    });
    return;
  }

  const typeArguments = idType.getAliasTypeArguments();
  if (typeArguments.length !== 1) {
    ctx.addWarning({
      title: "Can’t update call site",
      message: `Sorry, we can’t infer the table type of \`${idArg.getText()}\` (which is a \`${idType.getText()}\`).`,
      node: idArg,
    });
    return;
  }

  const tableName = typeArguments[0];
  if (!tableName.isStringLiteral()) {
    ctx.addWarning({
      title: "Can’t update call site",
      message: `Expected \`${idArg.getText()}\` to be an \`Id<T>\`, where \`T\` is a string literal, but got \`T = ${tableName.getText()}\` instead.`,
      node: idArg,
    });
    return;
  }

  call.insertArgument(0, tableName.getText());
  ctx.incrementChanges(call.getSourceFile().getFilePath());
}

/**
 * Awaiting this function in a loop allows us to make sure the user
 * can interrupt the codemod with CTRL+C
 */
async function allowInterrupt() {
  await new Promise((resolve) => setTimeout(resolve, 0));
}
