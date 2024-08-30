import { PropertyValidators, convexToJson } from "../../values/index.js";
import {
  AnyFunctionReference,
  FunctionReference,
  FunctionType,
} from "../api.js";
import { getFunctionAddress } from "../impl/actions_impl.js";
import { performAsyncSyscall } from "../impl/syscall.js";
import { DefaultFunctionArgs } from "../registration.js";
import {
  AppDefinitionAnalysis,
  ComponentDefinitionAnalysis,
  ComponentDefinitionType,
} from "./definition.js";

export const toReferencePath = Symbol.for("toReferencePath");

export function extractReferencePath(reference: any): string | null {
  return reference[toReferencePath] ?? null;
}

export function isFunctionHandle(s: string): boolean {
  return s.startsWith("function://");
}

/**
 * @internal
 */
export type FunctionHandle<
  Type extends FunctionType,
  Args extends DefaultFunctionArgs = any,
  ReturnType = any,
> = string & FunctionReference<Type, "internal", Args, ReturnType>;

/**
 * @internal
 */
export async function createFunctionHandle<
  Type extends FunctionType,
  Args extends DefaultFunctionArgs,
  ReturnType,
>(
  functionReference: FunctionReference<
    Type,
    "public" | "internal",
    Args,
    ReturnType
  >,
): Promise<FunctionHandle<Type, Args, ReturnType>> {
  const address = getFunctionAddress(functionReference);
  return await performAsyncSyscall("1.0/createFunctionHandle", { ...address });
}

interface ComponentExports {
  [key: string]: FunctionReference<any, any, any, any> | ComponentExports;
}

/**
 * @internal
 */
export interface InitCtx {}

/**
 * An object of this type should be the default export of a
 * convex.config.ts file in a component definition directory.
 *
 * @internal
 */ // eslint-disable-next-line @typescript-eslint/ban-types
export type ComponentDefinition<Exports extends ComponentExports = any> = {
  /**
   * Install a component with the given definition in this component definition.
   *
   * Takes a component definition, and an optional name.
   *
   * For editor tooling this method expects a {@link ComponentDefinition}
   * but at runtime the object that is imported will be a {@link ImportedComponentDefinition}
   */
  use<Definition extends ComponentDefinition<any>>(
    definition: Definition,
    options?: {
      name?: string;
    },
  ): InstalledComponent<Definition>;

  /**
   * @internal
   */
  __exports: Exports;
};

type ComponentDefinitionExports<T extends ComponentDefinition<any>> =
  T["__exports"];

/**
 * An object of this type should be the default export of a
 * convex.config.ts file in a component-aware convex directory.
 *
 * @internal
 */
export type AppDefinition = {
  /**
   * Install a component with the given definition in this component definition.
   *
   * Takes a component definition, an optional name, and the args it requires.
   *
   * For editor tooling this method expects a {@link ComponentDefinition}
   * but at runtime the object that is imported will be a {@link ImportedComponentDefinition}
   */
  use<Definition extends ComponentDefinition<any>>(
    definition: Definition,
    options?: {
      name?: string;
    },
  ): InstalledComponent<Definition>;
};

interface ExportTree {
  // Tree with serialized `Reference`s as leaves.
  [key: string]: string | ExportTree;
}

type CommonDefinitionData = {
  _isRoot: boolean;
  _childComponents: [
    string,
    ImportedComponentDefinition,
    Record<string, any> | null,
  ][];
  _exportTree: ExportTree;
};

type ComponentDefinitionData = CommonDefinitionData & {
  _args: PropertyValidators;
  _name: string;
  _onInitCallbacks: Record<string, (argsStr: string) => string>;
};
type AppDefinitionData = CommonDefinitionData;

/**
 * Used to refer to an already-installed component.
 */
class InstalledComponent<Definition extends ComponentDefinition<any>> {
  /**
   * @internal
   */
  _definition: Definition;

  /**
   * @internal
   */
  _name: string;

  /**
   * @internal
   */
  [toReferencePath]: string;

  constructor(definition: Definition, name: string) {
    this._definition = definition;
    this._name = name;
    this[toReferencePath] = `_reference/childComponent/${name}`;
  }

  get exports(): ComponentDefinitionExports<Definition> {
    return createExports(this._name, []);
  }
}

function createExports(name: string, pathParts: string[]): any {
  const handler: ProxyHandler<any> = {
    get(_, prop: string | symbol) {
      if (typeof prop === "string") {
        const newParts = [...pathParts, prop];
        return createExports(name, newParts);
      } else if (prop === toReferencePath) {
        let reference = `_reference/childComponent/${name}`;
        for (const part of pathParts) {
          reference += `/${part}`;
        }
        return reference;
      } else {
        return undefined;
      }
    },
  };
  return new Proxy({}, handler);
}

function use<Definition extends ComponentDefinition<any>>(
  this: CommonDefinitionData,
  definition: Definition,
  options?: {
    name?: string;
  },
): InstalledComponent<Definition> {
  // At runtime an imported component will have this shape.
  const importedComponentDefinition =
    definition as unknown as ImportedComponentDefinition;
  if (typeof importedComponentDefinition.componentDefinitionPath !== "string") {
    throw new Error(
      "Component definition does not have the required componentDefinitionPath property. This code only works in Convex runtime.",
    );
  }
  const name =
    options?.name ||
    importedComponentDefinition.componentDefinitionPath.split("/").pop()!;
  this._childComponents.push([name, importedComponentDefinition, {}]);
  return new InstalledComponent(definition, name);
}

// At runtime when you import a ComponentDefinition, this is all it is
/**
 * @internal
 */
export type ImportedComponentDefinition = {
  componentDefinitionPath: string;
};

function exportAppForAnalysis(
  this: ComponentDefinition<any> & AppDefinitionData,
): AppDefinitionAnalysis {
  const definitionType = { type: "app" as const };
  const childComponents = serializeChildComponents(this._childComponents);
  return {
    definitionType,
    childComponents: childComponents as any,
    httpMounts: {},
    exports: serializeExportTree(this._exportTree),
  };
}

function serializeExportTree(tree: ExportTree): any {
  const branch: any[] = [];
  for (const [key, child] of Object.entries(tree)) {
    let node;
    if (typeof child === "string") {
      node = { type: "leaf", leaf: child };
    } else {
      node = serializeExportTree(child);
    }
    branch.push([key, node]);
  }
  return { type: "branch", branch };
}

function serializeChildComponents(
  childComponents: [
    string,
    ImportedComponentDefinition,
    Record<string, any> | null,
  ][],
): {
  name: string;
  path: string;
  args: [string, { type: "value"; value: string }][] | null;
}[] {
  return childComponents.map(([name, definition, p]) => {
    let args: [string, { type: "value"; value: string }][] | null = null;
    if (p !== null) {
      args = [];
      for (const [name, value] of Object.entries(p)) {
        if (value !== undefined) {
          args.push([
            name,
            { type: "value", value: JSON.stringify(convexToJson(value)) },
          ]);
        }
      }
    }
    // we know that components carry this extra information
    const path = definition.componentDefinitionPath;
    if (!path)
      throw new Error(
        "no .componentPath for component definition " +
          JSON.stringify(definition, null, 2),
      );

    return {
      name: name!,
      path: path!,
      args,
    };
  });
}

function exportComponentForAnalysis(
  this: ComponentDefinition<any> & ComponentDefinitionData,
): ComponentDefinitionAnalysis {
  const args: [string, { type: "value"; value: string }][] = Object.entries(
    this._args,
  ).map(([name, validator]) => [
    name,
    {
      type: "value",
      value: JSON.stringify(validator.json),
    },
  ]);
  const definitionType: ComponentDefinitionType = {
    type: "childComponent" as const,
    name: this._name,
    args,
  };
  const childComponents = serializeChildComponents(this._childComponents);
  return {
    name: this._name,
    definitionType,
    childComponents: childComponents as any,
    httpMounts: {},
    exports: serializeExportTree(this._exportTree),
  };
}

// This is what is actually contained in a ComponentDefinition.
type RuntimeComponentDefinition = Omit<ComponentDefinition<any>, "__exports"> &
  ComponentDefinitionData & {
    export: () => ComponentDefinitionAnalysis;
  };
type RuntimeAppDefinition = AppDefinition &
  AppDefinitionData & {
    export: () => AppDefinitionAnalysis;
  };

/**
 * @internal
 */
// eslint-disable-next-line @typescript-eslint/ban-types
export function defineComponent<Exports extends ComponentExports = any>(
  name: string,
): ComponentDefinition<Exports> {
  const ret: RuntimeComponentDefinition = {
    _isRoot: false,
    _name: name,
    _args: {},
    _childComponents: [],
    _exportTree: {},
    _onInitCallbacks: {},

    export: exportComponentForAnalysis,
    use,

    // pretend to conform to ComponentDefinition, which temporarily expects __args
    ...({} as { __args: any; __exports: any }),
  };
  return ret as any as ComponentDefinition<Exports>;
}

/**
 * Experimental - DO NOT USE.
 */
// TODO Make this not experimental.
export function defineApp(): AppDefinition {
  const ret: RuntimeAppDefinition = {
    _isRoot: true,
    _childComponents: [],
    _exportTree: {},

    export: exportAppForAnalysis,
    use,
  };
  return ret as AppDefinition;
}

type AnyInterfaceType = {
  [key: string]: AnyInterfaceType;
} & AnyFunctionReference;
export type AnyComponentReference = Record<string, AnyInterfaceType>;

type AnyChildComponents = Record<string, AnyComponentReference>;

/**
 * @internal
 */
export function currentSystemUdfInComponent(
  componentId: string,
): AnyComponentReference {
  return {
    [toReferencePath]: `_reference/currentSystemUdfInComponent/${componentId}`,
  };
}

function createChildComponents(
  root: string,
  pathParts: string[],
): AnyChildComponents {
  const handler: ProxyHandler<object> = {
    get(_, prop: string | symbol) {
      if (typeof prop === "string") {
        const newParts = [...pathParts, prop];
        return createChildComponents(root, newParts);
      } else if (prop === toReferencePath) {
        if (pathParts.length < 1) {
          const found = [root, ...pathParts].join(".");
          throw new Error(
            `API path is expected to be of the form \`${root}.childComponent.functionName\`. Found: \`${found}\``,
          );
        }
        return `_reference/childComponent/` + pathParts.join("/");
      } else {
        return undefined;
      }
    },
  };
  return new Proxy({}, handler);
}

/**
 * @internal
 */
export const componentsGeneric = () => createChildComponents("components", []);

/**
 * @internal
 */
export type AnyComponents = AnyChildComponents;
