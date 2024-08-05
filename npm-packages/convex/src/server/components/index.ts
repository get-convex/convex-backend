import {
  Infer,
  ObjectType,
  PropertyValidators,
  convexToJson,
  jsonToConvex,
} from "../../values/index.js";
import {
  AnyFunctionReference,
  functionName,
  FunctionReference,
  FunctionType,
} from "../api.js";
import { performAsyncSyscall, performSyscall } from "../impl/syscall.js";
import { DefaultFunctionArgs, EmptyObject } from "../registration.js";
import {
  AppDefinitionAnalysis,
  ComponentDefinitionAnalysis,
  ComponentDefinitionType,
  HttpMount,
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
  const udfPath = (functionReference as any)[functionName];
  if (!udfPath) {
    throw new Error(`${functionReference as any} is not a FunctionReference`);
  }
  return await performAsyncSyscall("1.0/createFunctionHandle", { udfPath });
}

interface ComponentExports {
  [key: string]: FunctionReference<any, any, any, any> | ComponentExports;
}

/**
 * An object of this type should be the default export of a
 * component.config.ts file in a component definition directory.
 *
 * @internal
 */ // eslint-disable-next-line @typescript-eslint/ban-types
export type ComponentDefinition<
  Args extends PropertyValidators = EmptyObject,
  Exports extends ComponentExports = any,
> = {
  /**
   * Install a component with the given definition in this component definition.
   *
   * Takes a component definition, an optional name, and the args it requires.
   *
   * For editor tooling this method expects a {@link ComponentDefinition}
   * but at runtime the object that is imported will be a {@link ImportedComponentDefinition}
   */
  install<Definition extends ComponentDefinition<any, any>>(
    definition: Definition,
    options: {
      name?: string;
      // TODO we have to do the "arguments are optional if empty, otherwise required"
      args?: ObjectType<ComponentDefinitionArgs<Definition>>;
    },
  ): InstalledComponent<Definition>;

  mount(exports: ComponentExports): void;

  /**
   * Mount a component's HTTP router at a given path prefix.
   */
  mountHttp(pathPrefix: string, component: InstalledComponent<any>): void;

  // TODO this will be needed once components are responsible for building interfaces for themselves
  /**
   * @internal
   */
  __args: Args;

  /**
   * @internal
   */
  __exports: Exports;
};

type ComponentDefinitionArgs<T extends ComponentDefinition<any, any>> =
  T["__args"];
type ComponentDefinitionExports<T extends ComponentDefinition<any, any>> =
  T["__exports"];

/**
 * An object of this type should be the default export of a
 * app.config.ts file in a component definition directory.
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
  install<Definition extends ComponentDefinition<any, any>>(
    definition: Definition,
    options: {
      name?: string;
      args?: ObjectType<ComponentDefinitionArgs<Definition>>;
    },
  ): InstalledComponent<Definition>;

  mount(exports: ComponentExports): void;

  /**
   * Mount a component's HTTP router at a given path prefix.
   */
  mountHttp(pathPrefix: string, component: InstalledComponent<any>): void;
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
    Record<string, any>,
  ][];
  _httpMounts: Record<string, HttpMount>;
  _exportTree: ExportTree;
};

type ComponentDefinitionData = CommonDefinitionData & {
  _args: PropertyValidators;
  _name: string;
};
type AppDefinitionData = CommonDefinitionData;

/**
 * Used to refer to an already-installed component.
 */
class InstalledComponent<Definition extends ComponentDefinition<any, any>> {
  /**
   * @internal
   */
  _definition: Definition;

  /**
   * @internal
   */
  [toReferencePath]: string;

  constructor(
    definition: Definition,
    private _name: string,
  ) {
    this._definition = definition;
    this[toReferencePath] = `_reference/childComponent/${_name}`;
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

function install<Definition extends ComponentDefinition<any>>(
  this: CommonDefinitionData,
  definition: Definition,
  options: {
    name?: string;
    args?: Infer<ComponentDefinitionArgs<Definition>>;
  } = {},
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
    options.name ||
    importedComponentDefinition.componentDefinitionPath.split("/").pop()!;
  this._childComponents.push([
    name,
    importedComponentDefinition,
    options.args || {},
  ]);

  return new InstalledComponent(definition, name);
}

function mount(this: CommonDefinitionData, exports: any) {
  function visit(definition: CommonDefinitionData, path: string[], value: any) {
    const valueReference = value[toReferencePath];
    if (valueReference) {
      if (!path.length) {
        throw new Error("Empty export path");
      }
      let current = definition._exportTree;
      for (const part of path.slice(0, -1)) {
        let next = current[part];
        if (typeof next === "string") {
          throw new Error(
            `Mount path ${path.join(".")} collides with existing export`,
          );
        }
        if (!next) {
          next = {};
          current[part] = next;
        }
        current = next;
      }
      const last = path[path.length - 1];
      if (current[last]) {
        throw new Error(
          `Mount path ${path.join(".")} collides with existing export`,
        );
      }
      current[last] = valueReference;
    } else {
      for (const [key, child] of Object.entries(value)) {
        visit(definition, [...path, key], child);
      }
    }
  }
  if (exports[toReferencePath]) {
    throw new Error(`Cannot mount another component's exports at the root`);
  }
  visit(this, [], exports);
}

function mountHttp(
  this: CommonDefinitionData,
  pathPrefix: string,
  component: InstalledComponent<any>,
) {
  if (!pathPrefix.startsWith("/")) {
    throw new Error(`Path prefix '${pathPrefix}' does not start with a /`);
  }
  if (!pathPrefix.endsWith("/")) {
    throw new Error(`Path prefix '${pathPrefix}' must end with a /`);
  }
  if (this._httpMounts[pathPrefix]) {
    throw new Error(`Path '${pathPrefix}' is already mounted.`);
  }
  const path = extractReferencePath(component);
  if (!path) {
    throw new Error("`mountHttp` must be called with an `InstalledComponent`.");
  }
  this._httpMounts[pathPrefix] = path;
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
    httpMounts: this._httpMounts,
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
  childComponents: [string, ImportedComponentDefinition, Record<string, any>][],
): {
  name: string;
  path: string;
  args: [string, { type: "value"; value: string }][];
}[] {
  return childComponents.map(([name, definition, p]) => {
    const args: [string, { type: "value"; value: string }][] = [];
    for (const [name, value] of Object.entries(p)) {
      if (value !== undefined) {
        args.push([
          name,
          { type: "value", value: JSON.stringify(convexToJson(value)) },
        ]);
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
    httpMounts: this._httpMounts,
    exports: serializeExportTree(this._exportTree),
  };
}

// This is what is actually contained in a ComponentDefinition.
type RuntimeComponentDefinition = Omit<
  ComponentDefinition<any, any>,
  "__args" | "__exports"
> &
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
export function defineComponent<
  Args extends PropertyValidators = EmptyObject,
  Exports extends ComponentExports = any,
>(
  name: string,
  options: { args?: Args } = {},
): ComponentDefinition<Args, Exports> {
  const ret: RuntimeComponentDefinition = {
    _isRoot: false,
    _name: name,
    _args: options.args || {},
    _childComponents: [],
    _httpMounts: {},
    _exportTree: {},

    export: exportComponentForAnalysis,
    install,
    mount,
    mountHttp,

    // pretend to conform to ComponentDefinition, which temporarily expects __args
    ...({} as { __args: any; __exports: any }),
  };
  return ret as any as ComponentDefinition<Args, Exports>;
}

/**
 * Experimental - DO NOT USE.
 */
// TODO Make this not experimental.
export function defineApp(): AppDefinition {
  const ret: RuntimeAppDefinition = {
    _isRoot: true,
    _childComponents: [],
    _httpMounts: {},
    _exportTree: {},

    export: exportAppForAnalysis,
    install,
    mount,
    mountHttp,
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
 *
 * @internal
 */
export function createComponentArg(): (ctx: any, name: string) => any {
  return (ctx: any, name: string) => {
    const result = performSyscall("1.0/componentArgument", {
      name,
    });
    return (jsonToConvex(result) as any).value;
  };
}

/**
 * @internal
 */
export const appGeneric = () => createChildComponents("app", []);

/**
 * @internal
 */
export type AnyApp = AnyChildComponents;

/**
 * @internal
 */
export const componentGeneric = () => createChildComponents("component", []);

/**
 * @internal
 */
export type AnyComponent = AnyChildComponents;
