import type { TSESTree } from "@typescript-eslint/utils";
import { AST_NODE_TYPES } from "@typescript-eslint/utils";
import { createRule } from "../util.js";
import type ts from "typescript";

type MessageIds = "no-filter-in-query";

const TERMINAL_QUERY_METHODS = new Set([
  "collect",
  "take",
  "first",
  "unique",
  "paginate",
]);

const QUERY_BUILDER_CHAIN_METHODS = [
  // QueryInitializer
  "withIndex",
  "withSearchIndex",
  "order",
  // OrderedQuery / Query
  "filter",
] as const;

function unwrapExpression(expr: TSESTree.Expression): TSESTree.Expression {
  let current: TSESTree.Expression = expr;
  while (true) {
    // ignore `?` (e.g. `a?.b`)
    if (current.type === AST_NODE_TYPES.ChainExpression) {
      current = current.expression;
      continue;
    }

    // ignore `await`
    if (current.type === AST_NODE_TYPES.AwaitExpression) {
      current = current.argument;
      continue;
    }

    // ignore `!`
    if (current.type === AST_NODE_TYPES.TSNonNullExpression) {
      current = current.expression;
      continue;
    }

    // ignore `as`
    if (current.type === AST_NODE_TYPES.TSAsExpression) {
      current = current.expression;
      continue;
    }

    // ignore `<Type>` in `<Type>value`
    if (current.type === AST_NODE_TYPES.TSTypeAssertion) {
      current = current.expression;
      continue;
    }

    // Ignore generic arguments (e.g. `<T>` in `fn<T>()`)
    if (current.type === AST_NODE_TYPES.TSInstantiationExpression) {
      current = current.expression;
      continue;
    }

    // Ignore parentheses
    const anyCurrent =
      // `ParenthesizedExpression` isn't present in all @typescript-eslint AST_NODE_TYPES versions.
      current as any;
    if (anyCurrent.type === "ParenthesizedExpression") {
      current = anyCurrent.expression as TSESTree.Expression;
      continue;
    }
    return current;
  }
}

// Whether the node is a call to a function that collects a query
function isTerminalQueryCall(expr: TSESTree.Expression): boolean {
  const unwrapped = unwrapExpression(expr);
  if (unwrapped.type !== AST_NODE_TYPES.CallExpression) return false;

  const callee = unwrapped.callee;
  if (callee.type !== AST_NODE_TYPES.MemberExpression) return false;
  if (callee.property.type !== AST_NODE_TYPES.Identifier) return false;
  return TERMINAL_QUERY_METHODS.has(callee.property.name);
}

function isDbQueryChainFallback(expr: TSESTree.Expression): boolean {
  const unwrapped = unwrapExpression(expr);
  if (unwrapped.type !== AST_NODE_TYPES.CallExpression) return false;

  const callee = unwrapped.callee;
  if (callee.type !== AST_NODE_TYPES.MemberExpression) return false;
  if (callee.property.type !== AST_NODE_TYPES.Identifier) return false;

  const methodName = callee.property.name;
  if (TERMINAL_QUERY_METHODS.has(methodName)) {
    // `db.query(...).collect()` and friends return arrays / values, not a query builder.
    return false;
  }

  if (methodName === "query") {
    const baseObject = unwrapExpression(callee.object as TSESTree.Expression);
    // `ctx.db.query(...)`
    if (
      baseObject.type === AST_NODE_TYPES.MemberExpression &&
      baseObject.property.type === AST_NODE_TYPES.Identifier &&
      baseObject.property.name === "db"
    ) {
      return true;
    }
    // `db.query(...)` (DatabaseReader / DatabaseWriter passed around)
    if (
      baseObject.type === AST_NODE_TYPES.Identifier &&
      baseObject.name === "db"
    ) {
      return true;
    }
    // `db.privateSystem.query(...)` (system UDFs)
    if (
      baseObject.type === AST_NODE_TYPES.MemberExpression &&
      baseObject.property.type === AST_NODE_TYPES.Identifier &&
      baseObject.property.name === "privateSystem"
    ) {
      const innerObject = unwrapExpression(
        baseObject.object as TSESTree.Expression,
      );
      if (
        innerObject.type === AST_NODE_TYPES.Identifier &&
        innerObject.name === "db"
      ) {
        return true;
      }
      if (
        innerObject.type === AST_NODE_TYPES.MemberExpression &&
        innerObject.property.type === AST_NODE_TYPES.Identifier &&
        innerObject.property.name === "db"
      ) {
        return true;
      }
    }
    return false;
  }

  // Continue walking down chained calls like `.withIndex(...)`, `.order(...)`, etc.
  return isDbQueryChainFallback(callee.object as TSESTree.Expression);
}

export const noFilterInQuery = createRule<[], MessageIds>({
  name: "no-filter-in-query",
  meta: {
    type: "suggestion",
    docs: {
      description:
        "Warn when using `.filter()` on a Convex database query, since it can be inefficient.",
    },
    messages: {
      "no-filter-in-query":
        "Avoid calling `.filter()` on a Convex database query; it can be inefficient (see Convex query best practices). If you have a real reason to use it, disable with `// eslint-disable-next-line @convex-dev/no-filter-in-query`.",
    },
    schema: [],
  },
  defaultOptions: [],
  create: (context) => {
    const filename = context.getFilename();
    if (filename.includes("_generated")) {
      return {};
    }

    const services = context.sourceCode.parserServices;
    const checker = services?.program?.getTypeChecker?.();
    const tsNodeMap = services?.esTreeNodeToTSNodeMap;
    const hasTypeInfo = !!(checker && tsNodeMap);

    // Detect Convex query `.filter()` via call signature:
    // Convex query filter predicates take a `FilterBuilder` parameter, whereas
    // array `.filter()` takes an element/index/array predicate.
    function typeContainsSymbolNamed(type: ts.Type, name: string): boolean {
      const symbol = type.aliasSymbol ?? type.getSymbol();
      if (symbol?.name === name) return true;
      // Union flag = 1048576 = 1 << 20
      if ((type.flags & (1 << 20)) !== 0) {
        const union = type as ts.UnionType;
        return union.types.some((t) => typeContainsSymbolNamed(t, name));
      }
      // Intersection flag = 2097152 = 1 << 21
      if ((type.flags & (1 << 21)) !== 0) {
        const intersection = type as ts.IntersectionType;
        return intersection.types.some((t) => typeContainsSymbolNamed(t, name));
      }
      return false;
    }

    function isConvexQueryFilterCall(node: TSESTree.CallExpression): boolean {
      if (!checker || !tsNodeMap) return false;
      try {
        const tsCall = tsNodeMap.get(node);
        const sig = checker.getResolvedSignature(tsCall as any);
        if (!sig) return false;
        const params = sig.getParameters();
        if (params.length < 1) return false;
        const predicateParamType = checker.getTypeOfSymbolAtLocation(
          params[0]!,
          tsCall as any,
        );
        const predicateSigs = predicateParamType.getCallSignatures();
        if (predicateSigs.length === 0) return false;
        const predSig = predicateSigs[0]!;
        const predParams = predSig.getParameters();
        if (predParams.length < 1) return false;
        const qParamType = checker.getTypeOfSymbolAtLocation(
          predParams[0]!,
          tsCall as any,
        );
        return typeContainsSymbolNamed(qParamType, "FilterBuilder");
      } catch {
        return false;
      }
    }

    // Track query builder types using subtype checking (like `explicitTableIds`).
    // We extract canonical query builder types from `_generated/server` so that
    // cases like `((db as any).privateSystem as DatabaseReader).query(...)` are
    // still recognized as Convex queries.
    const queryVarSymbols = new Set<ts.Symbol>();
    const queryBuilderTypes: ts.Type[] = [];
    const queryBuilderTypeStrings = new Set<string>();

    function addQueryBuilderType(t: ts.Type) {
      if (!checker) return;
      const s = checker.typeToString(t);
      if (s === "any" || s === "unknown") return;
      if (queryBuilderTypeStrings.has(s)) return;
      queryBuilderTypeStrings.add(s);
      queryBuilderTypes.push(t);
    }

    function isPossiblyAssignableTo(source: ts.Type, target: ts.Type): boolean {
      if (!checker) return false;
      // For unions, accept if any constituent is assignable.
      if ((source.flags & (1 << 20)) !== 0) {
        // Union flag = 1048576 = 1 << 20
        const union = source as ts.UnionType;
        return union.types.some((t) => checker.isTypeAssignableTo(t, target));
      }
      return checker.isTypeAssignableTo(source, target);
    }

    function isConvexQueryBuilderType(t: ts.Type): boolean {
      if (!checker) return false;
      if (checker.typeToString(t) === "any") {
        return false;
      }
      return queryBuilderTypes.some((qt) => isPossiblyAssignableTo(t, qt));
    }

    function isIdentifierExplicitAnyType(id: TSESTree.Identifier): boolean {
      return (
        id.typeAnnotation?.typeAnnotation.type === AST_NODE_TYPES.TSAnyKeyword
      );
    }

    function getCallReturnTypes(fnType: ts.Type): ts.Type[] {
      if (!checker) return [];
      const sigs = fnType.getCallSignatures();
      if (sigs.length === 0) return [];
      return sigs.map((s) => s.getReturnType());
    }

    function getPropertyType(
      symbol: ts.Symbol,
      fallbackNode: ts.Node,
    ): ts.Type {
      if (!checker) {
        // Should never happen if we call this.
        return {} as ts.Type;
      }
      const decl =
        symbol.valueDeclaration ?? symbol.declarations?.[0] ?? fallbackNode;
      return checker.getTypeOfSymbolAtLocation(symbol, decl);
    }

    function seedQueryBuilderTypesFromGeneratedServer() {
      if (!checker || !services?.program) return;

      let anyDatabaseReader: ts.Type | null = null;
      let generatedServerSourceFile: ts.SourceFile | null = null;

      try {
        const sourceFiles = services.program.getSourceFiles();
        for (const sf of sourceFiles) {
          if (sf.fileName.includes("_generated/server")) {
            const sourceFileSymbol = checker.getSymbolAtLocation(sf);
            if (!sourceFileSymbol) continue;
            const exports = checker.getExportsOfModule(sourceFileSymbol);
            for (const exp of exports) {
              const type = checker.getDeclaredTypeOfSymbol(exp);
              const typeString = checker.typeToString(type);

              // Prefer an exported DatabaseReader-like type that has a `.query` method.
              if (
                (exp.name === "DatabaseReader" ||
                  exp.name === "GenericDatabaseReader" ||
                  typeString.includes("DatabaseReader") ||
                  typeString.includes("GenericDatabaseReader")) &&
                type.getProperty("query")
              ) {
                anyDatabaseReader = type;
                generatedServerSourceFile = sf;
                break;
              }

              // Otherwise, fall back to grabbing `.db` from a ctx type.
              if (
                typeString.includes("DatabaseReader") ||
                typeString.includes("GenericDatabaseReader")
              ) {
                const dbProp = type.getProperty("db");
                if (dbProp) {
                  anyDatabaseReader = checker.getTypeOfSymbolAtLocation(
                    dbProp,
                    sf,
                  );
                  generatedServerSourceFile = sf;
                  break;
                }
              }
            }
            break;
          }
        }
      } catch {
        // If we can't get the types, we'll fall back to AST pattern matching.
      }

      if (!anyDatabaseReader || !generatedServerSourceFile) return;

      const queryProp = anyDatabaseReader.getProperty("query");
      if (!queryProp) return;

      const queryFnType = getPropertyType(queryProp, generatedServerSourceFile);
      const seedTypes = getCallReturnTypes(queryFnType);
      for (const t of seedTypes) addQueryBuilderType(t);

      // Expand: add return types of common chain methods from the seed types so
      // `.filter()` after `.withIndex()` / `.order()` is still covered.
      const queue = [...seedTypes];
      const seen = new Set<string>();
      while (queue.length > 0) {
        const t = queue.pop()!;
        const key = checker.typeToString(t);
        if (seen.has(key)) continue;
        seen.add(key);

        for (const methodName of QUERY_BUILDER_CHAIN_METHODS) {
          const prop = t.getProperty(methodName);
          if (!prop) continue;
          const fnType = getPropertyType(prop, generatedServerSourceFile);
          for (const rt of getCallReturnTypes(fnType)) {
            const s = checker.typeToString(rt);
            if (s === "any" || s === "unknown") continue;
            addQueryBuilderType(rt);
            queue.push(rt);
          }
        }
      }
    }

    if (checker && tsNodeMap) {
      seedQueryBuilderTypesFromGeneratedServer();
    }

    return {
      CallExpression(node: TSESTree.CallExpression) {
        if (node.callee.type !== AST_NODE_TYPES.MemberExpression) return;

        const memberExpr = node.callee;
        if (memberExpr.property.type !== AST_NODE_TYPES.Identifier) return;

        // Track `const q = db.query(...);` style query builders so we can
        // later recognize `q.filter(...)`.
        if (
          checker &&
          tsNodeMap &&
          memberExpr.property.name === "query" &&
          node.parent?.type === AST_NODE_TYPES.VariableDeclarator &&
          node.parent.id.type === AST_NODE_TYPES.Identifier
        ) {
          const varDecl = node.parent;
          const initExpr = varDecl.init as TSESTree.Expression | null;
          if (initExpr && isDbQueryChainFallback(initExpr)) {
            try {
              const idTsNode = tsNodeMap.get(varDecl.id);
              const sym = checker.getSymbolAtLocation(idTsNode as any);
              if (
                sym &&
                !isIdentifierExplicitAnyType(varDecl.id as TSESTree.Identifier)
              ) {
                queryVarSymbols.add(sym);
              }

              const initTsNode = tsNodeMap.get(unwrapExpression(initExpr));
              addQueryBuilderType(checker.getTypeAtLocation(initTsNode as any));
            } catch {
              // ignore
            }
          }
        }

        if (memberExpr.property.name !== "filter") return;

        const receiver = memberExpr.object as TSESTree.Expression;

        // Don’t warn for array `.filter()` after running a query, e.g.
        // `(await db.query(...).collect()).filter(...)`.
        if (isTerminalQueryCall(receiver)) return;

        // If type info is available, prefer determining if this is Convex query
        // `.filter()` via the predicate signature (FilterBuilder parameter).
        if (isConvexQueryFilterCall(node)) {
          context.report({
            node: memberExpr.property,
            messageId: "no-filter-in-query",
          });
          return;
        }

        let isConvexQuery = false;
        let shouldUseAstFallback = !hasTypeInfo;
        if (hasTypeInfo) {
          try {
            const receiverUnwrapped = unwrapExpression(receiver);
            const receiverTsNode = tsNodeMap.get(receiverUnwrapped);
            const receiverType = checker.getTypeAtLocation(
              receiverTsNode as any,
            );
            const receiverTypeString = checker.typeToString(receiverType);
            const hasUsefulReceiverType =
              receiverTypeString !== "any" && receiverTypeString !== "unknown";

            if (receiverUnwrapped.type === AST_NODE_TYPES.Identifier) {
              const sym = checker.getSymbolAtLocation(receiverTsNode as any);
              if (sym && queryVarSymbols.has(sym)) {
                isConvexQuery = true;
              }
            }
            if (hasUsefulReceiverType) {
              isConvexQuery =
                isConvexQuery || isConvexQueryBuilderType(receiverType);
              // Type-aware check gave a concrete verdict; don't let AST heuristic override it.
              shouldUseAstFallback = false;
            }
          } catch {
            // Fall back below.
            shouldUseAstFallback = true;
          }
        }

        // Fallback when linting is non type-aware or type info is inconclusive.
        if (!isConvexQuery && shouldUseAstFallback) {
          isConvexQuery = isDbQueryChainFallback(receiver);
        }

        if (isConvexQuery) {
          context.report({
            node: memberExpr.property,
            messageId: "no-filter-in-query",
          });
        }
      },
    };
  },
});
