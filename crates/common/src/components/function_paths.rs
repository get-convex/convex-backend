use sync_types::{
    CanonicalizedUdfPath,
    UdfPath,
};

use super::{
    component_definition_path::ComponentDefinitionPath,
    ComponentId,
};

pub struct ComponentDefinitionFunctionPath {
    pub component: ComponentDefinitionPath,
    pub udf_path: UdfPath,
}

pub struct ComponentFunctionPath {
    pub component: ComponentId,
    pub udf_path: UdfPath,
}

pub struct CanonicalizedComponentFunctionPath {
    pub component: ComponentId,
    pub udf_path: CanonicalizedUdfPath,
}
