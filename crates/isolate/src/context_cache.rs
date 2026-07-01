use std::{
    collections::{
        HashMap,
        HashSet,
    },
    sync::Arc,
};

use common::{
    components::CanonicalizedComponentModulePath,
    interval::IntervalSet,
    runtime::Runtime,
    types::TabletIndexName,
};
use database::TransactionReadSet;
use deno_core::v8::{
    self,
    scope,
};
use fastrace::{
    local::LocalSpan,
    Event,
};
use parking_lot::Mutex;
use value::{
    sha256::Sha256Digest,
    TableName,
    TableNamespace,
};

use crate::{
    client::{
        Request,
        RequestType,
    },
    metrics::create_context_timer,
    module_map::ModuleMap,
};

pub struct ContextCache {
    fresh_context: Option<v8::Global<v8::Context>>,
    saved_database_udf_contexts: HashMap<
        CanonicalizedComponentModulePath,
        (v8::Global<v8::Context>, ModuleMap, ContextReadSet),
    >,
    is_under_memory_pressure: bool,
    cached_contexts: Arc<CachedContexts>,
}

/// A mirror of the cache keys present in a `ContextCache`.
/// This struct is `Send + Sync` so that it can be used by the isolate
/// scheduler.
pub struct CachedContexts {
    inner: Mutex<CachedContextsInner>,
}

struct CachedContextsInner {
    saved_database_udf_contexts: HashSet<CanonicalizedComponentModulePath>,
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
    pub fn new() -> Self {
        Self {
            fresh_context: None,
            saved_database_udf_contexts: HashMap::new(),
            is_under_memory_pressure: false,
            cached_contexts: Arc::new(CachedContexts {
                inner: Mutex::new(CachedContextsInner {
                    saved_database_udf_contexts: HashSet::new(),
                }),
            }),
        }
    }

    pub(crate) fn prepare(&mut self, isolate: &mut v8::Isolate) {
        if self.fresh_context.is_none() && !self.is_under_memory_pressure {
            scope!(let scope, isolate);
            let context = make_context(scope);
            self.fresh_context = Some(v8::Global::new(scope, context));
        }
    }

    /// Report that the isolate's memory usage has exceeded a threshold.
    /// If there are any saved contexts, they will be scheduled to be cleared
    /// unless they are immediately reused.
    ///
    /// Returns whether there are any contexts that would be cleared.
    pub(crate) fn report_memory_pressure(&mut self, is_under_memory_pressure: bool) -> bool {
        if self.saved_database_udf_contexts.is_empty() {
            // Ignore the signal in this case as there is nothing we can do.
            self.is_under_memory_pressure = false;
            return false;
        }
        self.is_under_memory_pressure = is_under_memory_pressure;
        true
    }

    pub(crate) fn clear(&mut self) {
        self.fresh_context = None;
        self.saved_database_udf_contexts.clear();
        self.cached_contexts
            .inner
            .lock()
            .saved_database_udf_contexts
            .clear();
    }

    pub(crate) fn get_or_create_fresh_context<'s>(
        &mut self,
        scope: &v8::PinScope<'s, '_, ()>,
    ) -> v8::Local<'s, v8::Context> {
        let fresh_context = self.fresh_context.take();
        if self.is_under_memory_pressure {
            LocalSpan::add_event(Event::new("cleared_contexts_under_memory_pressure"));
            self.clear();
        }
        if let Some(context) = fresh_context {
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
        self.saved_database_udf_contexts
            .insert(module_path.clone(), (context, module_map, read_set));
        self.cached_contexts
            .inner
            .lock()
            .saved_database_udf_contexts
            .insert(module_path);
    }

    pub(crate) fn take_reused_context(
        &mut self,
        module_path: &CanonicalizedComponentModulePath,
    ) -> Option<(v8::Global<v8::Context>, ModuleMap, ContextReadSet)> {
        self.cached_contexts
            .inner
            .lock()
            .saved_database_udf_contexts
            .remove(module_path);
        self.saved_database_udf_contexts.remove(module_path)
    }

    pub fn cached_contexts(&self) -> &Arc<CachedContexts> {
        &self.cached_contexts
    }
}

impl CachedContexts {
    pub fn can_serve_request<RT: Runtime>(&self, request: &Request<RT>) -> bool {
        let this = self.inner.lock();
        match &request.inner {
            RequestType::Udf { request: inner, .. } if inner.path_and_args.reuse_context() => {
                request
                    .module()
                    .is_some_and(|m| this.saved_database_udf_contexts.contains(&m))
            },
            // Prefer routing other requests to isolates that don't have warmed contexts
            _ => this.saved_database_udf_contexts.is_empty(),
        }
    }
}

fn make_context<'s>(scope: &v8::PinScope<'s, '_, ()>) -> v8::Local<'s, v8::Context> {
    let _create_context_timer = create_context_timer();
    v8::Context::new(scope, v8::ContextOptions::default())
}
