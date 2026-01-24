//! This module contains the implementation of both synchronous and
//! async ops. Unlike syscalls, these functions are present in *every*
//! environment, but the environment may decide not to implement their
//! functionality, causing a runtime error.

mod console;
mod crypto;
mod database;
mod environment_variables;
mod errors;
mod http;
mod random;
mod storage;
mod stream;
mod structured_clone;
mod subtle_crypto;
mod text;
mod time;
mod validate_args;
mod validate_returns;

use std::collections::BTreeMap;

use ::errors::ErrorMetadata;
use anyhow::anyhow;
use bytes::Bytes;
use common::{
    log_lines::LogLevel,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        EnvVarName,
        EnvVarValue,
    },
};
use deno_core::{
    v8,
    ModuleSpecifier,
};
use rand_chacha::ChaCha12Rng;
use sourcemap::SourceMap;
use structured_clone::op_structured_clone;
use uuid::Uuid;
use validate_returns::op_validate_returns;
use value::{
    heap_size::WithHeapSize,
    NamespacedTableMapping,
};

use self::{
    console::{
        op_console_message,
        op_console_time_end,
        op_console_time_log,
        op_console_time_start,
        op_console_trace,
    },
    crypto::{
        op_crypto_get_random_values,
        op_crypto_random_uuid,
    },
    database::op_get_table_mapping,
    environment_variables::op_environment_variables_get,
    errors::{
        op_error_stack,
        op_throw_uncatchable_developer_error,
    },
    http::{
        async_op_fetch,
        async_op_parse_multi_part,
        op_headers_get_mime_type,
        op_headers_normalize_name,
        op_url_get_url_info,
        op_url_get_url_search_param_pairs,
        op_url_stringify_url_search_params,
        op_url_update_url_info,
    },
    random::op_random,
    storage::{
        async_op_storage_get,
        async_op_storage_store,
    },
    stream::{
        async_op_stream_read_part,
        op_stream_create,
        op_stream_extend,
    },
    text::{
        op_atob,
        op_btoa,
        op_text_encoder_cleanup,
        op_text_encoder_decode,
        op_text_encoder_decode_single,
        op_text_encoder_encode,
        op_text_encoder_encode_into,
        op_text_encoder_new_decoder,
        op_text_encoder_normalize_label,
    },
    time::{
        async_op_sleep,
        op_now,
    },
    validate_args::op_validate_args,
};
use crate::{
    environment::{
        crypto_rng::CryptoRng,
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    helpers::to_rust_string,
    metrics,
    request_scope::{
        ReadableStream,
        StreamListener,
        TextDecoderResource,
    },
};

pub trait OpProvider<'b> {
    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng>;
    fn crypto_rng(&mut self) -> anyhow::Result<CryptoRng>;
    fn scope(&mut self) -> v8::PinScope<'_, 'b>;
    fn lookup_source_map(
        &mut self,
        specifier: &ModuleSpecifier,
    ) -> anyhow::Result<Option<SourceMap>>;
    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()>;
    fn console_timers(
        &mut self,
    ) -> anyhow::Result<&mut WithHeapSize<BTreeMap<String, UnixTimestamp>>>;
    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp>;
    fn unix_timestamp_non_deterministic(&mut self) -> anyhow::Result<UnixTimestamp>;

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()>;

    fn create_stream(&mut self) -> anyhow::Result<Uuid>;
    fn extend_stream(
        &mut self,
        id: Uuid,
        bytes: Option<Bytes>,
        new_done: bool,
    ) -> anyhow::Result<()>;
    fn new_stream_listener(
        &mut self,
        stream_id: Uuid,
        listener: StreamListener,
    ) -> anyhow::Result<()>;

    fn create_text_decoder(&mut self, decoder: TextDecoderResource) -> anyhow::Result<Uuid>;
    fn get_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<&mut TextDecoderResource>;
    fn remove_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<TextDecoderResource>;

    fn get_environment_variable(&mut self, name: EnvVarName)
        -> anyhow::Result<Option<EnvVarValue>>;

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping>;
}

impl<'a, 's: 'a, 'i, RT: Runtime, E: IsolateEnvironment<RT>> OpProvider<'i>
    for ExecutionScope<'a, 's, 'i, RT, E>
{
    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        let state = self.state_mut()?;
        state.environment.rng()
    }

    fn crypto_rng(&mut self) -> anyhow::Result<CryptoRng> {
        let state = self.state_mut()?;
        state.environment.crypto_rng()
    }

    fn lookup_source_map(
        &mut self,
        specifier: &ModuleSpecifier,
    ) -> anyhow::Result<Option<SourceMap>> {
        ExecutionScope::lookup_source_map(self, specifier)
    }

    fn scope(&mut self) -> v8::PinScope<'_, 'i> {
        self.as_mut_ref()
    }

    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        state.environment.trace(level, messages)?;
        Ok(())
    }

    fn console_timers(
        &mut self,
    ) -> anyhow::Result<&mut WithHeapSize<BTreeMap<String, UnixTimestamp>>> {
        let state = self.state_mut()?;
        Ok(&mut state.console_timers)
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        let state = self.state_mut()?;
        state.environment.unix_timestamp()
    }

    fn unix_timestamp_non_deterministic(&mut self) -> anyhow::Result<UnixTimestamp> {
        let state = self.state_mut()?;
        Ok(state.unix_timestamp_non_deterministic())
    }

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        state.environment.start_async_op(request, resolver)
    }

    fn create_stream(&mut self) -> anyhow::Result<Uuid> {
        self.state_mut()?.create_stream()
    }

    fn extend_stream(
        &mut self,
        id: Uuid,
        bytes: Option<Bytes>,
        new_done: bool,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        if let Some(bytes) = &bytes
            && let Some(request_stream_state) = state.request_stream_state.as_mut()
            && request_stream_state.stream_id() == id
        {
            request_stream_state.track_bytes_read(bytes.len());
        }
        state.streams.mutate(&id, |stream| -> anyhow::Result<()> {
            let Some(Ok(ReadableStream { parts, done })) = stream else {
                anyhow::bail!("unrecognized stream id {id}");
            };
            if *done {
                anyhow::bail!("stream {id} is already done");
            }
            if let Some(bytes) = bytes {
                parts.push_back(bytes);
            }
            if new_done {
                *done = true;
            }
            Ok(())
        })?;
        self.update_stream_listeners()?;
        Ok(())
    }

    fn new_stream_listener(
        &mut self,
        stream_id: Uuid,
        listener: StreamListener,
    ) -> anyhow::Result<()> {
        if self
            .state_mut()?
            .stream_listeners
            .insert(stream_id, listener)
            .is_some()
        {
            anyhow::bail!("cannot read from the same stream twice");
        }
        self.update_stream_listeners()
    }

    fn create_text_decoder(&mut self, decoder: TextDecoderResource) -> anyhow::Result<Uuid> {
        self.state_mut()?.create_text_decoder(decoder)
    }

    fn get_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<&mut TextDecoderResource> {
        self.state_mut()?.get_text_decoder(uuid)
    }

    fn remove_text_decoder(&mut self, uuid: &Uuid) -> anyhow::Result<TextDecoderResource> {
        self.state_mut()?.remove_text_decoder(uuid)
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        let state = self.state_mut()?;
        state.environment.get_environment_variable(name)
    }

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping> {
        let state = self.state_mut()?;
        state.environment.get_all_table_mappings()
    }
}

pub fn run_op<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) -> anyhow::Result<()> {
    if args.length() < 1 {
        // This must be a bug in our `udf-runtime` code, not a developer error.
        anyhow::bail!("op(op_name, ...) takes at least one argument");
    }
    let op_name: v8::Local<v8::String> = args.get(0).try_into()?;
    let op_name = to_rust_string(&provider.scope(), &op_name)?;

    let timer = metrics::op_timer(&op_name);
    match &op_name[..] {
        "throwUncatchableDeveloperError" => {
            op_throw_uncatchable_developer_error(provider, args, rv)?
        },
        "console/message" => op_console_message(provider, args, rv)?,
        "console/trace" => op_console_trace(provider, args, rv)?,
        "console/timeStart" => op_console_time_start(provider, args, rv)?,
        "console/timeLog" => op_console_time_log(provider, args, rv)?,
        "console/timeEnd" => op_console_time_end(provider, args, rv)?,
        "error/stack" => op_error_stack(provider, args, rv)?,
        "random" => op_random(provider, args, rv)?,
        "now" => op_now(provider, args, rv)?,
        "url/getUrlInfo" => op_url_get_url_info(provider, args, rv)?,
        "url/getUrlSearchParamPairs" => op_url_get_url_search_param_pairs(provider, args, rv)?,
        "url/stringifyUrlSearchParams" => op_url_stringify_url_search_params(provider, args, rv)?,
        "url/updateUrlInfo" => op_url_update_url_info(provider, args, rv)?,
        "headers/getMimeType" => op_headers_get_mime_type(provider, args, rv)?,
        "headers/normalizeName" => op_headers_normalize_name(provider, args, rv)?,
        "stream/create" => op_stream_create(provider, args, rv)?,
        "stream/extend" => op_stream_extend(provider, args, rv)?,
        "textEncoder/encode" => op_text_encoder_encode(provider, args, rv)?,
        "textEncoder/encodeInto" => op_text_encoder_encode_into(provider, args, rv)?,
        "textEncoder/decodeSingle" => op_text_encoder_decode_single(provider, args, rv)?,
        "textEncoder/decode" => op_text_encoder_decode(provider, args, rv)?,
        "textEncoder/newDecoder" => op_text_encoder_new_decoder(provider, args, rv)?,
        "textEncoder/cleanup" => op_text_encoder_cleanup(provider, args, rv)?,
        "textEncoder/normalizeLabel" => op_text_encoder_normalize_label(provider, args, rv)?,
        "atob" => op_atob(provider, args, rv)?,
        "btoa" => op_btoa(provider, args, rv)?,
        "structuredClone" => op_structured_clone(provider, args.get(1), rv)?,
        "environmentVariables/get" => op_environment_variables_get(provider, args, rv)?,
        "getTableMapping" => op_get_table_mapping(provider, args, rv)?,
        "validateArgs" => op_validate_args(provider, args, rv)?,
        "validateReturns" => op_validate_returns(provider, args, rv)?,

        "crypto/randomUUID" => op_crypto_random_uuid(provider, args, rv)?,
        "crypto/getRandomValues" => op_crypto_get_random_values(provider, args, rv)?,
        "crypto/subtle/decrypt" => subtle_crypto::op_crypto_subtle_decrypt(provider, args, rv)?,
        "crypto/subtle/deriveBits" => {
            subtle_crypto::op_crypto_subtle_derive_bits(provider, args, rv)?
        },
        "crypto/subtle/deriveKey" => {
            subtle_crypto::op_crypto_subtle_derive_key(provider, args, rv)?
        },
        "crypto/subtle/digest" => subtle_crypto::op_crypto_subtle_digest(provider, args, rv)?,
        "crypto/subtle/encrypt" => subtle_crypto::op_crypto_subtle_encrypt(provider, args, rv)?,
        "crypto/subtle/exportKey" => {
            subtle_crypto::op_crypto_subtle_export_key(provider, args, rv)?
        },
        "crypto/subtle/generateKey" => {
            subtle_crypto::op_crypto_subtle_generate_key(provider, args, rv)?
        },
        "crypto/subtle/importKey" => {
            subtle_crypto::op_crypto_subtle_import_key(provider, args, rv)?
        },
        "crypto/subtle/sign" => subtle_crypto::op_crypto_subtle_sign(provider, args, rv)?,
        "crypto/subtle/unwrapKey" => {
            subtle_crypto::op_crypto_subtle_unwrap_key(provider, args, rv)?
        },
        "crypto/subtle/verify" => subtle_crypto::op_crypto_subtle_verify(provider, args, rv)?,
        "crypto/subtle/wrapKey" => subtle_crypto::op_crypto_subtle_wrap_key(provider, args, rv)?,
        _ => {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnknownOperation",
                format!("Unknown operation {op_name}")
            ));
        },
    }
    timer.finish();
    Ok(())
}

pub fn start_async_op<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) -> anyhow::Result<()> {
    if args.length() < 1 {
        anyhow::bail!("asyncOp(op, ...args) takes at least one argument");
    }
    let scope = provider.scope();
    let op_name: v8::Local<v8::String> = args.get(0).try_into()?;
    let op_name = to_rust_string(&scope, &op_name)?;

    let resolver = v8::PromiseResolver::new(&scope)
        .ok_or_else(|| anyhow!("Failed to create PromiseResolver"))?;
    let resolver = v8::Global::new(&scope, resolver);

    match &op_name[..] {
        "fetch" => async_op_fetch(provider, args, resolver.clone())?,
        "form/parseMultiPart" => async_op_parse_multi_part(provider, args, resolver.clone())?,
        "sleep" => async_op_sleep(provider, args, resolver.clone())?,
        "storage/store" => async_op_storage_store(provider, args, resolver.clone())?,
        "storage/get" => async_op_storage_get(provider, args, resolver.clone())?,
        "stream/readPart" => async_op_stream_read_part(provider, args, resolver.clone())?,
        _ => {
            anyhow::bail!(ErrorMetadata::bad_request(
                "UnknownAsyncOperation",
                format!("Unknown async operation {op_name}")
            ));
        },
    };

    // TODO: ideally we should not need to clone `resolver`, but
    // `OpProvider::scope` returns a scope with a restricted lifetime
    let scope = provider.scope();
    let promise = v8::Local::new(&scope, resolver).get_promise(&scope);
    rv.set(promise.into());
    Ok(())
}
