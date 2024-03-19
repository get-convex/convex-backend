use std::collections::{
    BTreeMap,
    HashMap,
};

use common::errors::JsError;
use deno_core::{
    v8::{
        self,
    },
    ModuleSpecifier,
};

use super::{
    client::PendingAsyncSyscall,
    environment::Environment,
    PromiseId,
};

pub struct ContextState {
    pub module_map: ModuleMap,
    pub unhandled_promise_rejections: HashMap<v8::Global<v8::Promise>, v8::Global<v8::Value>>,
    pub pending_dynamic_imports: Vec<(ModuleSpecifier, v8::Global<v8::PromiseResolver>)>,

    pub next_promise_id: u64,
    pub pending_async_syscalls: Vec<PendingAsyncSyscall>,
    pub promise_resolvers: HashMap<PromiseId, v8::Global<v8::PromiseResolver>>,

    pub environment: Box<dyn Environment>,

    pub failure: Option<ContextFailure>,
}

impl ContextState {
    pub fn new(environment: Box<dyn Environment>) -> Self {
        Self {
            module_map: ModuleMap::new(),
            unhandled_promise_rejections: HashMap::new(),
            pending_dynamic_imports: vec![],

            next_promise_id: 0,
            pending_async_syscalls: vec![],
            promise_resolvers: HashMap::new(),

            environment,

            failure: None,
        }
    }
}

pub enum ContextFailure {
    UncatchableDeveloperError(JsError),
}

pub struct ModuleMap {
    pub modules: BTreeMap<ModuleSpecifier, v8::Global<v8::Module>>,
    pub by_v8_module: HashMap<v8::Global<v8::Module>, ModuleSpecifier>,
}

impl ModuleMap {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            by_v8_module: HashMap::new(),
        }
    }

    pub fn contains_module(&self, name: &ModuleSpecifier) -> bool {
        self.modules.contains_key(name)
    }

    pub fn register(
        &mut self,
        name: ModuleSpecifier,
        module: v8::Global<v8::Module>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.modules.contains_key(&name),
            "Module already registered"
        );
        self.modules.insert(name.clone(), module.clone());
        self.by_v8_module.insert(module, name);
        Ok(())
    }
}
