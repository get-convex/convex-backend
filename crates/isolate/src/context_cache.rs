use std::collections::HashMap;

use common::{
    components::CanonicalizedComponentModulePath,
    interval::IntervalSet,
    types::TabletIndexName,
};
use database::TransactionReadSet;
use deno_core::v8::{
    self,
    scope,
};
use value::{
    sha256::Sha256Digest,
    TableName,
    TableNamespace,
};

use crate::{
    metrics::create_context_timer,
    module_map::ModuleMap,
};

pub struct ContextCache {
    fresh_context: Option<v8::Global<v8::Context>>,
    saved_contexts: HashMap<
        CanonicalizedComponentModulePath,
        (v8::Global<v8::Context>, ModuleMap, ContextReadSet),
    >,
}

pub(crate) struct ContextReadSet {
    pub read_set: TransactionReadSet,
    pub range_hashes: Vec<(
        TableNamespace,
        TabletIndexName,
        TableName,
        IntervalSet,
        Sha256Digest,
    )>,
}

impl ContextCache {
    pub(crate) fn new() -> Self {
        Self {
            fresh_context: None,
            saved_contexts: HashMap::new(),
        }
    }

    pub(crate) fn prepare(&mut self, isolate: &mut v8::Isolate) {
        if self.fresh_context.is_none() {
            scope!(let scope, isolate);
            let context = make_context(scope);
            self.fresh_context = Some(v8::Global::new(scope, context));
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::new();
    }

    pub(crate) fn get_or_create_fresh_context<'s>(
        &mut self,
        scope: &v8::PinScope<'s, '_, ()>,
    ) -> v8::Local<'s, v8::Context> {
        if let Some(context) = self.fresh_context.take() {
            v8::Local::new(scope, context)
        } else {
            make_context(scope)
        }
    }

    pub(crate) fn save_context(
        &mut self,
        module_path: CanonicalizedComponentModulePath,
        context: v8::Global<v8::Context>,
        module_map: ModuleMap,
        read_set: ContextReadSet,
    ) {
        self.saved_contexts
            .insert(module_path, (context, module_map, read_set));
    }

    pub(crate) fn take_reused_context(
        &mut self,
        module_path: &CanonicalizedComponentModulePath,
    ) -> Option<(v8::Global<v8::Context>, ModuleMap, ContextReadSet)> {
        self.saved_contexts.remove(module_path)
    }
}

fn make_context<'s>(scope: &v8::PinScope<'s, '_, ()>) -> v8::Local<'s, v8::Context> {
    let _create_context_timer = create_context_timer();
    v8::Context::new(scope, v8::ContextOptions::default())
}
