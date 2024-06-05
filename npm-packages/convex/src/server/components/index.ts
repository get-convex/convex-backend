import {
  Infer,
  ObjectType,
  PropertyValidators,
  convexToJson,
} from "../../values/index.js";
import {
  AppDefinitionAnalysis,
  ComponentDefinitionAnalysis,
  ComponentDefinitionType,
} from "./definition";

type ComponentArgsMethod<
  IsRoot extends boolean,
  Args extends PropertyValidators,
> = IsRoot extends false
  ? {
      args<AdditionalArgs extends PropertyValidators>(
        args: AdditionalArgs,
      ): ComponentDefinition<IsRoot, Args & AdditionalArgs>;
    }
  : // eslint-disable-next-line @typescript-eslint/ban-types
    {};

type CommonComponentsDefinition<
  IsRoot extends boolean,
  Args extends PropertyValidators,
> = {
  install<Definition extends ComponentDefinition<false, any>>(
    name: string,
    definition: Definition,
    args: ObjectType<ExtractArgs<Definition>>,
  ): ComponentDefinition<IsRoot, Args>;
};

type ComponentDefinition<
  IsRoot extends boolean,
  Args extends PropertyValidators,
> = CommonComponentsDefinition<IsRoot, Args> &
  ComponentArgsMethod<IsRoot, Args>;

type CommonDefinitionData = {
  _isRoot: boolean;
  _childComponents: [string, ImportedComonentDefinition, Record<string, any>][];
};
type ComponentDefinitionData = CommonDefinitionData & {
  _args: PropertyValidators;
  _name: string;
};
type AppDefinitionData = CommonDefinitionData;

type ExtractArgs<T> = T extends ComponentDefinition<any, infer P> ? P : never;

type DefineComponent = (
  name: string,
  // eslint-disable-next-line @typescript-eslint/ban-types
) => ComponentDefinition<false, {}>;

type DefineApp = <Args extends PropertyValidators>() => ComponentDefinition<
  true,
  Args
>;

function componentArgsBuilder(
  this: ComponentDefinition<false, any> & ComponentDefinitionData,
  additionalArgs: PropertyValidators,
): ComponentDefinition<any, any> & ComponentDefinitionData {
  return { ...this, _args: { ...this._args, ...additionalArgs } };
}

function installBuilder<Definition extends ComponentDefinition<false, any>>(
  this: ComponentDefinition<false, any> & ComponentDefinitionData,
  name: string,
  definition: Definition,
  args: Infer<ExtractArgs<Definition>>,
) {
  const importedComponentDefinition =
    definition as unknown as ImportedComonentDefinition;
  return {
    ...this,
    _childComponents: [
      ...this._childComponents,
      [name, importedComponentDefinition, args],
    ],
  };
}

// Injected by the bundler
type BundlerAssignedComponentData = {
  _componentPath: string;
};

// At runtime when you import a ComponentDefinition, this is all it is
type ImportedComonentDefinition = {
  componentDefinitionPath: string;
};

function exportAppForAnalysis(
  this: ComponentDefinition<true, any> &
    ComponentDefinitionData &
    BundlerAssignedComponentData,
): AppDefinitionAnalysis {
  if (!this._isRoot) {
    throw new Error(
      "`exportComponentForAnalysis()` must run on a root component.",
    );
  }
  const componentPath = this._componentPath;
  if (!componentPath === undefined) {
    throw new Error(
      `ComponentPath not found in ${JSON.stringify(this, null, 2)}`,
    );
  }
  const definitionType = { type: "app" as const };
  const childComponents = serializeChildComponents(this._childComponents);

  return {
    // this is a component path. It will need to be provided by the bundler somehow.
    // An esbuild plugin needs to take over to do this during bundling.
    path: componentPath,
    definitionType,
    childComponents: childComponents as any,
    exports: { type: "branch", branch: [] },
  };
}

function serializeChildComponents(
  childComponents: [string, ImportedComonentDefinition, Record<string, any>][],
): {
  name: string;
  path: string;
  args: [string, { type: "value"; value: string }][];
}[] {
  return childComponents.map(([name, definition, p]) => {
    const args: [string, { type: "value"; value: string }][] = [];
    for (const [name, value] of Object.entries(p)) {
      args.push([
        name,
        { type: "value", value: JSON.stringify(convexToJson(value)) },
      ]);
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
  this: ComponentDefinition<any, any> &
    ComponentDefinitionData &
    BundlerAssignedComponentData,
): ComponentDefinitionAnalysis {
  if (this._isRoot) {
    throw new Error(
      "`exportComponentForAnalysis()` cannot run on a root component.",
    );
  }
  const componentPath = this._componentPath;
  if (!componentPath === undefined) {
    throw new Error(
      `ComponentPath not found in ${JSON.stringify(this, null, 2)}`,
    );
  }
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
    // this is a component path. It will need to be provided by the bundler somehow.
    // An esbuild plugin needs to take over to do this during bundling.
    path: componentPath,
    definitionType,
    childComponents: childComponents as any,
    exports: { type: "branch", branch: [] },
  };
}

function defineComponentImpl(
  componentPath: string, // secret first argument inserted by the bundler
  name: string,
  // eslint-disable-next-line @typescript-eslint/ban-types
): ComponentDefinition<any, any> &
  ComponentDefinitionData & {
    export: () => ComponentDefinitionAnalysis;
  } & BundlerAssignedComponentData {
  if (name === undefined) {
    throw new Error(
      "defineComponentImpl needs its secret first argument filled in by the bundler",
    );
  }
  return {
    _isRoot: false,
    _name: name,
    _args: {},
    _childComponents: [],
    _componentPath: componentPath,
    args: componentArgsBuilder,
    export: exportComponentForAnalysis,
    install: installBuilder,
  };
}

function defineAppImpl(
  componentPath: string, // secret first argument inserted by the bundler
): ComponentDefinition<true, any> &
  AppDefinitionData & {
    export: () => AppDefinitionAnalysis;
  } & BundlerAssignedComponentData {
  if (componentPath === undefined) {
    throw new Error(
      "defineComponentImpl needs its secret first argument filled in by the bundler",
    );
  }
  return {
    _isRoot: true,
    _childComponents: [],
    _componentPath: componentPath,
    export: exportAppForAnalysis,
    install: installBuilder,
  };
}

/**
 * @internal
 */
export const defineComponent =
  defineComponentImpl as unknown as DefineComponent;
/**
 * @internal
 */
export const defineApp = defineAppImpl as unknown as DefineApp;
