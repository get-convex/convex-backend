use std::{
    collections::HashMap,
    sync::Arc,
};

use deno_core::{
    v8,
    ModuleSpecifier,
};
use model::modules::module_versions::{
    FullModuleSource,
    SourceMap,
};

pub type ModuleId = usize;

/// All of the modules currently loaded, indexed by name and by V8 handle.
pub struct ModuleMap {
    modules: Vec<ModuleInfo>,
    by_name: HashMap<ModuleSpecifier, ModuleId>,
    by_handle: HashMap<v8::Global<v8::Module>, ModuleId>,
}

struct ModuleInfo {
    pub name: ModuleSpecifier,
    pub handle: v8::Global<v8::Module>,
    pub module_source: Arc<FullModuleSource>,
}

impl ModuleMap {
    pub fn new() -> Self {
        Self {
            modules: vec![],
            by_name: HashMap::new(),
            by_handle: HashMap::new(),
        }
    }

    pub fn name_by_handle(&self, handle: &v8::Global<v8::Module>) -> Option<&ModuleSpecifier> {
        let id = self.by_handle.get(handle)?;
        let info = &self.modules[*id];
        Some(&info.name)
    }

    pub fn handle_by_id(&self, id: ModuleId) -> Option<v8::Global<v8::Module>> {
        self.modules.get(id).map(|m| m.handle.clone())
    }

    pub fn get_by_name(&self, specifier: &ModuleSpecifier) -> Option<ModuleId> {
        self.by_name.get(specifier).cloned()
    }

    pub fn source_map(&self, id: ModuleId) -> Option<&SourceMap> {
        self.modules[id].module_source.source_map.as_ref()
    }

    pub fn register(
        &mut self,
        name: &ModuleSpecifier,
        handle: v8::Global<v8::Module>,
        module_source: Arc<FullModuleSource>,
    ) -> ModuleId {
        let id = self.modules.len();

        let info = ModuleInfo {
            name: name.to_owned(),
            handle: handle.clone(),
            module_source,
        };
        self.modules.push(info);
        self.by_name.insert(name.to_owned(), id);
        self.by_handle.insert(handle, id);

        id
    }
}
