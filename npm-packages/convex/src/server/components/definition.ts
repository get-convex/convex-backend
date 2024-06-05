// These reflect server types.
export type ComponentDefinitionExport = {
  name: string;
  // how will we figure this out?
  path: string;
  definitionType: {
    type: "childComponent";
    name: string;
    args: [string, { type: "value"; value: string }][];
  };
  childComponents: [];
  exports: { type: "branch"; branch: [] };
};

// These reflect server types.
// type ComponentDefinitionType
export type ComponentDefinitionType = {
  type: "childComponent";
  name: string;
  args: [string, { type: "value"; value: string }][];
};
export type AppDefinitionType = { type: "app" };

type ComponentInstantiation = {
  name: string;
  // This is a ComponentPath.
  path: string;
  args: [string, { type: "value"; value: string }][];
};

type ComponentExport =
  | { type: "branch"; branch: ComponentExport[] }
  | { type: "leaf"; leaf: string };

// The type expected from the internal .export()
// method of a component or app definition.
export type ComponentDefinitionAnalysis = {
  name: string;
  path: string;
  definitionType: ComponentDefinitionType;
  childComponents: ComponentInstantiation[];
  exports: ComponentExport;
};
export type AppDefinitionAnalysis = {
  path: string;
  definitionType: AppDefinitionType;
  childComponents: ComponentInstantiation[];
  exports: ComponentExport;
};
