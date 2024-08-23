use std::collections::{
    btree_map::Entry,
    BTreeMap,
};

use common::{
    bootstrap_model::components::definition::ComponentExport,
    components::Reference,
};
use errors::ErrorMetadata;
use sync_types::CanonicalizedUdfPath;

use super::types::EvaluatedComponentDefinition;
use crate::modules::module_versions::Visibility;

pub fn add_file_based_routing(evaluated: &mut EvaluatedComponentDefinition) -> anyhow::Result<()> {
    for (module_path, module) in &evaluated.functions {
        let mut identifiers = vec![];
        let stripped = module_path.clone().strip();

        identifiers.extend(stripped.components());
        for function in &module.functions {
            if function.visibility != Some(Visibility::Public) {
                continue;
            }
            let mut path = identifiers.clone();
            path.push(function.name.clone().into());
            let (last, prefix) = path.split_last().unwrap();

            let mut current = &mut evaluated.definition.exports;
            for identifier in prefix {
                let current_node = current
                    .entry(identifier.clone())
                    .or_insert_with(|| ComponentExport::Branch(BTreeMap::new()));
                current = match current_node {
                    ComponentExport::Branch(ref mut branch) => branch,
                    ComponentExport::Leaf(..) => anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidExport",
                        format!(
                            "Path {module_path:?}:{} conflicts with existing export",
                            function.name
                        )
                    )),
                }
            }
            match current.entry(last.clone()) {
                Entry::Vacant(e) => {
                    let path =
                        CanonicalizedUdfPath::new(module_path.clone(), function.name.clone());
                    let reference = Reference::Function(path);
                    e.insert(ComponentExport::Leaf(reference));
                },
                Entry::Occupied(_) => anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidExport",
                    format!(
                        "Path {module_path:?}:{} conflicts with existing export",
                        function.name
                    )
                )),
            }
        }
    }
    Ok(())
}
