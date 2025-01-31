import { Parser } from "acorn";
import { ValidatorJSON } from "convex/values";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import {
  Node,
  ExpressionNode,
  isUndefined,
} from "elements/ObjectEditor/ast/types";
import { walkPatchDocuments } from "elements/ObjectEditor/ast/walkPatchDocuments";
import { Walker } from "elements/ObjectEditor/ast/ast";
import { walkMultipleDocuments } from "elements/ObjectEditor/ast/walkMultipleDocuments";
import { IdWalker } from "elements/ObjectEditor/ast/astIds";

export type WalkAstOptions = {
  validator?: ValidatorJSON;
} & (
  | { mode: "addDocuments" | "editDocument" | "patchDocuments" }
  | {
      mode: "editField";
      allowTopLevelUndefined?: boolean;
    }
);

export const walkAst = (code: string, options: WalkAstOptions) => {
  const ast = Parser.parse(`(${code})`, {
    ecmaVersion: 2020,
    locations: true,
  }) as Node;

  // Peel off the unneeded nodes off the AST.
  const programBody = extractExpressionFromAST(ast);

  const { value, errors } = extractValueFromExpression(programBody, options);

  const idFinder = new IdWalker();

  const ids = idFinder.walk(programBody.expression);

  return {
    value,
    errors,
    ids,
  };
};

function extractExpressionFromAST(ast: Node) {
  if (ast.type !== "Program" || ast.body.length !== 1) {
    throw new Error("Invalid AST, expected Program with one node in body");
  }

  const programBody = ast.body[0];
  if (programBody.type !== "ExpressionStatement") {
    throw new Error("Invalid AST, expected ExpressionStatement");
  }
  return programBody;
}

function extractValueFromExpression(
  programBody: ExpressionNode,
  options: WalkAstOptions,
) {
  const { mode, validator } = options;
  const { expression } = programBody;

  if (
    mode === "editField" &&
    options.allowTopLevelUndefined &&
    isUndefined(expression)
  ) {
    return { value: UNDEFINED_PLACEHOLDER, errors: [] };
  }

  if (mode === "patchDocuments") {
    return walkPatchDocuments(expression, validator);
  }

  if (mode === "addDocuments" && expression.type === "ArrayExpression") {
    return walkMultipleDocuments(expression, validator);
  }

  // We are not editing multiple documents, so we can walk as a single node.
  const walker = new Walker({ validator });

  const isTopLevel = mode === "editDocument" || mode === "addDocuments";
  return walker.walk(expression, isTopLevel);
}
