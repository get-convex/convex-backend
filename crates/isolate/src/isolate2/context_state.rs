use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    mem,
};

use anyhow::Context;
use bytes::Bytes;
use common::{
    errors::{
        report_error_sync,
        JsError,
    },
    runtime::UnixTimestamp,
};
use deno_core::{
    v8,
    ModuleSpecifier,
};
use uuid::Uuid;
use value::heap_size::WithHeapSize;

use super::{
    client::{
        Pending,
        PendingAsyncOp,
        PendingAsyncSyscall,
        PendingDynamicImport,
    },
    environment::Environment,
    PromiseId,
};
use crate::{
    environment::UncatchableDeveloperError,
    ops::CryptoOps,
    request_scope::{
        ReadableStream,
        StreamListener,
        TextDecoderResource,
    },
};

pub struct ContextState {
    pub module_map: ModuleMap,
    pub unhandled_promise_rejections: HashMap<v8::Global<v8::Promise>, v8::Global<v8::Value>>,

    pub next_promise_id: PromiseId,
    pub promise_resolvers: BTreeMap<PromiseId, v8::Global<v8::PromiseResolver>>,

    pub pending_async_syscalls: Vec<PendingAsyncSyscall>,
    pub pending_async_ops: Vec<PendingAsyncOp>,
    pub pending_dynamic_imports: Vec<PendingDynamicImport>,

    pub blob_parts: WithHeapSize<BTreeMap<Uuid, Bytes>>,

    pub streams: WithHeapSize<BTreeMap<Uuid, anyhow::Result<ReadableStream>>>,
    pub stream_listeners: WithHeapSize<BTreeMap<Uuid, StreamListener>>,

    pub console_timers: WithHeapSize<BTreeMap<String, UnixTimestamp>>,

    // This is not wrapped in `WithHeapSize` so we can return `&mut TextDecoderStream`.
    // Additionally, `TextDecoderResource` should have a fairly small heap size.
    pub text_decoders: BTreeMap<uuid::Uuid, TextDecoderResource>,

    pub environment: Box<dyn Environment>,

    pub failure: Option<ContextFailure>,
}

impl ContextState {
    pub fn new(environment: Box<dyn Environment>) -> Self {
        Self {
            module_map: ModuleMap::new(),
            unhandled_promise_rejections: HashMap::new(),

            next_promise_id: 0,
            promise_resolvers: BTreeMap::new(),

            pending_async_syscalls: vec![],
            pending_async_ops: vec![],
            pending_dynamic_imports: vec![],

            blob_parts: BTreeMap::new().into(),

            streams: BTreeMap::new().into(),
            stream_listeners: BTreeMap::new().into(),

            console_timers: BTreeMap::new().into(),

            text_decoders: BTreeMap::new(),

            environment,

            failure: None,
        }
    }

    pub fn take_pending(&mut self) -> Pending {
        Pending {
            async_syscalls: mem::take(&mut self.pending_async_syscalls),
            async_ops: mem::take(&mut self.pending_async_ops),
            dynamic_imports: mem::take(&mut self.pending_dynamic_imports),
        }
    }

    pub fn register_promise(&mut self, promise: v8::Global<v8::PromiseResolver>) -> PromiseId {
        let id = self.next_promise_id;
        self.next_promise_id += 1;
        self.promise_resolvers.insert(id, promise);
        id
    }

    pub fn take_promise(
        &mut self,
        id: PromiseId,
    ) -> anyhow::Result<v8::Global<v8::PromiseResolver>> {
        self.promise_resolvers
            .remove(&id)
            .context("Promise resolver not found")
    }

    pub fn create_blob_part(&mut self, bytes: Bytes) -> anyhow::Result<Uuid> {
        let uuid = CryptoOps::random_uuid(self.environment.rng()?)?;
        self.blob_parts.insert(uuid, bytes);
        Ok(uuid)
    }

    pub fn get_blob_part(&self, id: &Uuid) -> Option<Bytes> {
        self.blob_parts.get(id).cloned()
    }

    pub fn create_stream(&mut self) -> anyhow::Result<Uuid> {
        let id = CryptoOps::random_uuid(self.environment.rng()?)?;
        self.streams.insert(id, Ok(ReadableStream::default()));
        Ok(id)
    }

    pub fn extend_stream(
        &mut self,
        id: Uuid,
        bytes: Option<Bytes>,
        new_done: bool,
    ) -> anyhow::Result<()> {
        let new_part_id = match bytes {
            Some(bytes) => Some(self.create_blob_part(bytes)?),
            None => None,
        };
        self.streams.mutate(&id, |stream| -> anyhow::Result<()> {
            let Some(Ok(ReadableStream { parts, done })) = stream else {
                anyhow::bail!("unrecognized stream id {id}");
            };
            if *done {
                anyhow::bail!("stream {id} is already done");
            }
            if let Some(new_part_id) = new_part_id {
                parts.push_back(new_part_id);
            }
            if new_done {
                *done = true;
            }
            Ok(())
        })?;
        Ok(())
    }

    pub fn new_stream_listener(
        &mut self,
        id: Uuid,
        listener: StreamListener,
    ) -> anyhow::Result<()> {
        if self.stream_listeners.insert(id, listener).is_some() {
            anyhow::bail!("cannot read from the same stream twice");
        }
        Ok(())
    }

    pub fn create_text_decoder(&mut self, resource: TextDecoderResource) -> anyhow::Result<Uuid> {
        let id = CryptoOps::random_uuid(self.environment.rng()?)?;
        self.text_decoders.insert(id, resource);
        Ok(id)
    }

    pub fn get_text_decoder(
        &mut self,
        decoder_id: &uuid::Uuid,
    ) -> anyhow::Result<&mut TextDecoderResource> {
        let decoder = self
            .text_decoders
            .get_mut(decoder_id)
            .ok_or_else(|| anyhow::anyhow!("Text decoder resource not found"))?;
        Ok(decoder)
    }

    pub fn remove_text_decoder(
        &mut self,
        decoder_id: &uuid::Uuid,
    ) -> anyhow::Result<TextDecoderResource> {
        let decoder = self
            .text_decoders
            .remove(decoder_id)
            .ok_or_else(|| anyhow::anyhow!("Text decoder resource not found"))?;
        Ok(decoder)
    }

    pub(crate) fn fail(&mut self, err: anyhow::Error) {
        if self.failure.is_some() {
            report_error_sync(&mut anyhow::anyhow!(
                "termination after already terminated: {err:?}"
            ));
            return;
        }

        self.failure = Some(match err.downcast::<UncatchableDeveloperError>() {
            Ok(err) => ContextFailure::UncatchableDeveloperError(err.js_error),
            Err(err) => ContextFailure::SystemError(err),
        });
    }
}

#[derive(Debug)]
pub enum ContextFailure {
    UncatchableDeveloperError(JsError),
    SystemError(anyhow::Error),
}

struct LoadedModule {
    pub handle: v8::Global<v8::Module>,
    pub source_map: Option<String>,
}

pub struct ModuleMap {
    modules: BTreeMap<ModuleSpecifier, LoadedModule>,
    by_v8_module: HashMap<v8::Global<v8::Module>, ModuleSpecifier>,
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

    pub fn lookup_module(&self, name: &ModuleSpecifier) -> Option<&v8::Global<v8::Module>> {
        self.modules.get(name).map(|m| &m.handle)
    }

    pub fn lookup_by_v8_module(&self, handle: &v8::Global<v8::Module>) -> Option<&ModuleSpecifier> {
        self.by_v8_module.get(handle)
    }

    pub fn lookup_source_map(&self, name: &ModuleSpecifier) -> Option<&str> {
        self.modules.get(name).and_then(|m| m.source_map.as_deref())
    }

    pub fn register(
        &mut self,
        name: ModuleSpecifier,
        v8_module: v8::Global<v8::Module>,
        source_map: Option<String>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.modules.contains_key(&name),
            "Module already registered"
        );
        let module = LoadedModule {
            handle: v8_module.clone(),
            source_map,
        };
        self.modules.insert(name.clone(), module);
        self.by_v8_module.insert(v8_module, name);
        Ok(())
    }
}
